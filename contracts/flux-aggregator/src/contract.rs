use cosmwasm_std::{
    log, to_binary, Api, Binary, CanonicalAddr, Env, Extern, HandleResponse, HumanAddr,
    InitResponse, Querier, QueryRequest, StdError, StdResult, Storage, Uint128, WasmMsg, WasmQuery,
};
use cw20::{BalanceResponse, Cw20ReceiveMsg};
use link_token::msg::{HandleMsg as LinkMsg, QueryMsg as LinkQuery};
use owned::contract::{get_owner, handle_accept_ownership, init as owned_init};
use utils::median::calculate_median;

use crate::{error::*, msg::*, state::*};

static RESERVE_ROUNDS: u128 = 2;
static MAX_ORACLE_COUNT: u128 = 77;
static ROUND_MAX: u32 = u32::MAX;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    if msg.min_submission_value > msg.max_submission_value {
        return ContractErr::MinGreaterThanMax.std_err();
    }
    oracle_addresses(&mut deps.storage).save(&vec![])?;
    recorded_funds(&mut deps.storage).save(&Funds::default())?;
    reporting_round_id(&mut deps.storage).save(&0)?;

    let link = deps.api.canonical_address(&msg.link)?;
    let validator = deps.api.canonical_address(&msg.validator)?;

    owned_init(deps, env.clone(), owned::msg::InitMsg {})?;

    config(&mut deps.storage).save(&State {
        link,
        validator,
        payment_amount: msg.payment_amount,
        min_submission_count: 0,
        max_submission_count: 0,
        restart_delay: 0,
        timeout: msg.timeout,
        decimals: msg.decimals,
        description: msg.description.clone(),
        min_submission_value: msg.min_submission_value,
        max_submission_value: msg.max_submission_value,
    })?;

    rounds(&mut deps.storage).save(
        &0_u32.to_be_bytes(), // TODO: this should be improved
        &Round {
            answer: None,
            started_at: None,
            updated_at: None,
            answered_in_round: env.block.time as u32 - msg.timeout,
        },
    )?;
    latest_round_id(&mut deps.storage).save(&0)?;

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Submit {
            round_id,
            submission,
        } => handle_submit(deps, env, round_id, submission),
        HandleMsg::ChangeOracles {
            removed,
            added,
            added_admins,
            min_submissions,
            max_submissions,
            restart_delay,
        } => handle_change_oracles(
            deps,
            env,
            removed,
            added,
            added_admins,
            min_submissions,
            max_submissions,
            restart_delay,
        ),
        HandleMsg::WithdrawPayment {
            oracle,
            recipient,
            amount,
        } => handle_withdraw_payment(deps, env, oracle, recipient, amount),
        HandleMsg::WithdrawFunds { recipient, amount } => {
            handle_withdraw_funds(deps, env, recipient, amount)
        }
        HandleMsg::TransferAdmin { oracle, new_admin } => {
            handle_transfer_admin(deps, env, oracle, new_admin)
        }
        HandleMsg::AcceptAdmin { oracle } => handle_accept_admin(deps, env, oracle),
        HandleMsg::RequestNewRound {} => handle_request_new_round(deps, env),
        HandleMsg::SetRequesterPermissions {
            requester,
            authorized,
            delay,
        } => handle_set_requester_permissions(deps, env, requester, authorized, delay),
        HandleMsg::UpdateFutureRounds {
            payment_amount,
            min_submissions,
            max_submissions,
            restart_delay,
            timeout,
        } => handle_update_future_rounds(
            deps,
            env,
            payment_amount,
            min_submissions,
            max_submissions,
            restart_delay,
            timeout,
        ),
        HandleMsg::UpdateAvailableFunds {} => handle_update_available_funds(deps, env),
        HandleMsg::SetValidator { validator } => handle_set_validator(deps, env, validator),
        HandleMsg::Receive(receive_msg) => handle_receive(deps, env, receive_msg),
        HandleMsg::TransferOwnership { to } => handle_transfer_ownership(deps, env, to),
        HandleMsg::AcceptOwnership {} => handle_accept_ownership(deps, env),
    }
}

pub fn handle_submit<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    round_id: u32,
    submission: Uint128,
) -> StdResult<HandleResponse> {
    let State {
        min_submission_value,
        max_submission_value,
        min_submission_count,
        max_submission_count,
        restart_delay,
        timeout,
        payment_amount,
        ..
    } = config_read(&deps.storage).load()?;
    if submission < min_submission_value {
        return ContractErr::UnderMin.std_err();
    }
    if submission > max_submission_value {
        return ContractErr::OverMax.std_err();
    }
    let sender_addr = deps.api.canonical_address(&env.message.sender)?;
    let messages = vec![];
    let mut logs = vec![];
    let timestamp = env.block.time;
    let sender_key = sender_addr.as_slice();
    let round_key = &round_id.to_be_bytes();

    let mut oracle = oracles_read(&deps.storage).load(sender_key)?;

    let rr_id = reporting_round_id_read(&deps.storage).load()?;
    validate_oracle_round(&deps.storage, &oracle, round_id, rr_id, timestamp)?;

    let mut round = rounds_read(&deps.storage)
        .may_load(round_key)?
        .unwrap_or_default();
    let mut round_details: RoundDetails;

    // if new round and delay requirement is met
    if round_id == rr_id + 1
        && (oracle.last_started_round.is_none()
            || round_id > oracle.last_started_round.unwrap() + restart_delay)
    {
        // update round info if timed out
        let timed_out_round = prev_round_id(round_id)?;
        if timed_out(&deps.storage, timed_out_round, env.block.time).unwrap_or(false) {
            let prev_round_id = prev_round_id(timed_out_round)?;
            let prev_round = rounds_read(&deps.storage).load(&prev_round_id.to_be_bytes())?;
            rounds(&mut deps.storage).update(&timed_out_round.to_be_bytes(), |round| {
                Ok(Round {
                    answer: prev_round.answer,
                    answered_in_round: prev_round.answered_in_round,
                    updated_at: Some(timestamp),
                    ..round.unwrap()
                })
            })?;
            details(&mut deps.storage).remove(&timed_out_round.to_be_bytes());
        }
        reporting_round_id(&mut deps.storage).save(&round_id)?;
        round_details = RoundDetails {
            submissions: vec![],
            max_submissions: max_submission_count,
            min_submissions: min_submission_count,
            timeout,
            payment_amount,
        };
        round.started_at = Some(timestamp);
        rounds(&mut deps.storage).save(round_key, &round)?;
        logs.extend_from_slice(&[
            log("action", "new round"),
            log("started by", &env.message.sender),
            log("started at", timestamp),
        ]);

        oracle.last_started_round = Some(round_id);
    } else {
        round_details = details_read(&deps.storage).load(round_key)?;
    }
    // record submission
    if !is_accepting_submissions(&round_details) {
        return ContractErr::NotAcceptingSubmissions.std_err();
    }
    round_details.submissions.push(submission);
    oracle.last_reported_round = Some(round_id);
    oracle.latest_submission = Some(submission);

    // update round answer
    if (round_details.submissions.len() as u32) >= round_details.min_submissions {
        let mut submissions = round_details
            .submissions
            .iter()
            .map(|submission| submission.u128())
            .collect::<Vec<u128>>();
        let new_answer =
            calculate_median(&mut submissions).map_err(|_| ContractErr::NoSubmissions.std())?;
        rounds(&mut deps.storage).save(
            round_key,
            &Round {
                answer: Some(Uint128(new_answer)),
                started_at: round.started_at,
                updated_at: Some(timestamp),
                answered_in_round: round_id,
            },
        )?;
        latest_round_id(&mut deps.storage).save(&round_id)?;
        logs.extend_from_slice(&[
            log("action", "answer updated"),
            log("current", Uint128(new_answer)),
            log("round_id", round_id),
        ]);

        // TODO: send new value to validator
        // messages.push(FlaggingValidatorMsg::ValidateAnwer)
    }
    // pay oracle
    let payment = round_details.payment_amount;
    recorded_funds(&mut deps.storage).update(|funds| {
        Ok(Funds {
            available: (funds.available - payment)?,
            allocated: funds.allocated + payment,
        })
    })?;
    oracle.withdrawable += payment;

    oracles(&mut deps.storage).save(sender_key, &oracle)?;

    // save or delete round details
    if (round_details.submissions.len() as u32) < round_details.max_submissions {
        details(&mut deps.storage).save(round_key, &round_details)?;
    } else {
        details(&mut deps.storage).remove(round_key);
    }

    Ok(HandleResponse {
        messages,
        log: logs,
        data: None,
    })
}

fn validate_oracle_round<S: Storage>(
    storage: &S,
    oracle: &OracleStatus,
    round_id: u32,
    rr_id: u32,
    timestamp: u64,
) -> StdResult<()> {
    if oracle.starting_round == 0 {
        return ContractErr::OracleNotEnabled.std_err();
    }
    if oracle.starting_round > round_id {
        return ContractErr::OracleNotYetEnabled.std_err();
    }
    if oracle.ending_round < round_id {
        return ContractErr::NoLongerAllowed.std_err();
    }
    if oracle
        .last_reported_round
        .map_or(false, |id| id >= round_id)
    {
        return ContractErr::ReportingPreviousRound.std_err();
    }
    let rr = rounds_read(storage).load(&rr_id.to_be_bytes())?;
    let unanswered = round_id + 1 == rr_id && rr.updated_at.is_none();
    if round_id != rr_id && round_id != rr_id + 1 && !unanswered {
        return ContractErr::InvalidRound.std_err();
    }
    if round_id != 1 && !is_supersedable(storage, prev_round_id(round_id)?, timestamp)? {
        return ContractErr::NotSupersedable.std_err();
    }
    Ok(())
}

fn is_supersedable<S: Storage>(storage: &S, round_id: u32, timestamp: u64) -> StdResult<bool> {
    let round = rounds_read(storage).load(&round_id.to_be_bytes())?;
    Ok(round.updated_at.unwrap() > 0 || timed_out(storage, round_id, timestamp)?)
}

fn timed_out<S: Storage>(storage: &S, round_id: u32, timestamp: u64) -> StdResult<bool> {
    let started_at = rounds_read(storage)
        .load(&round_id.to_be_bytes())?
        .started_at;
    let round_details = details_read(storage).may_load(&round_id.to_be_bytes())?;
    if let Some(round_details) = round_details {
        let timeout = round_details.timeout as u64;
        Ok(started_at.is_some() && timeout > 0 && started_at.unwrap() + timeout < timestamp)
    } else {
        Ok(false)
    }
}

fn initialize_new_round<S: Storage>(
    storage: &mut S,
    round_id: u32,
    timestamp: u64,
) -> StdResult<()> {
    // update round info if timed out
    let timed_out_round = prev_round_id(round_id)?;
    if timed_out(storage, timed_out_round, timestamp)? {
        let prev_round_id = prev_round_id(timed_out_round)?;
        let prev_round = rounds_read(storage).load(&prev_round_id.to_be_bytes())?;
        rounds(storage).update(&timed_out_round.to_be_bytes(), |round| {
            Ok(Round {
                answer: prev_round.answer,
                answered_in_round: prev_round.answered_in_round,
                updated_at: Some(timestamp),
                ..round.unwrap()
            })
        })?;
        details(storage).remove(&timed_out_round.to_be_bytes());
    }

    reporting_round_id(storage).save(&round_id)?;
    let State {
        min_submission_count,
        max_submission_count,
        timeout,
        payment_amount,
        ..
    } = config_read(storage).load()?;
    details(storage).save(
        &round_id.to_be_bytes(),
        &RoundDetails {
            submissions: vec![],
            max_submissions: max_submission_count,
            min_submissions: min_submission_count,
            timeout,
            payment_amount,
        },
    )?;
    rounds(storage).update(&round_id.to_be_bytes(), |round| {
        Ok(Round {
            started_at: Some(timestamp),
            ..round.unwrap_or_default()
        })
    })?;

    Ok(())
}

fn prev_round_id(round_id: u32) -> StdResult<u32> {
    round_id
        .checked_sub(1)
        .ok_or_else(|| StdError::underflow(round_id, 1))
}

fn is_accepting_submissions(details: &RoundDetails) -> bool {
    details.max_submissions != 0
}

#[allow(clippy::too_many_arguments)]
pub fn handle_change_oracles<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    removed: Vec<HumanAddr>,
    added: Vec<HumanAddr>,
    added_admins: Vec<HumanAddr>,
    min_submissions: u32,
    max_submissions: u32,
    restart_delay: u32,
) -> StdResult<HandleResponse> {
    validate_ownership(deps, &env)?;

    for oracle in removed {
        let oracle = deps.api.canonical_address(&oracle)?;
        remove_oracle(&mut deps.storage, oracle)?;
    }

    if added.len() != added_admins.len() {
        return ContractErr::OracleAdminCountMismatch.std_err();
    }
    let oracle_count: u128 = get_oracle_count(deps)?.into();
    let new_count = oracle_count + added.len() as u128;
    if new_count > MAX_ORACLE_COUNT {
        return ContractErr::MaxOraclesAllowed.std_err();
    }

    for (oracle, admin) in added.iter().zip(added_admins) {
        let oracle = deps.api.canonical_address(oracle)?;
        let admin = deps.api.canonical_address(&admin)?;
        add_oracle(&mut deps.storage, oracle, admin)?;
    }

    let State {
        payment_amount,
        timeout,
        ..
    } = config_read(&deps.storage).load()?;
    let msg = WasmMsg::Execute {
        contract_addr: env.contract.address,
        msg: to_binary(&HandleMsg::UpdateFutureRounds {
            payment_amount,
            min_submissions,
            max_submissions,
            restart_delay,
            timeout,
        })?,
        send: vec![],
    };

    Ok(HandleResponse {
        messages: vec![msg.into()],
        log: vec![], // TODO: add logs
        data: None,
    })
}

fn remove_oracle<S: Storage>(storage: &mut S, oracle: CanonicalAddr) -> StdResult<()> {
    // TODO: is this needed?
    // let reporting_round = reporting_round_id_read(storage).load()?;
    let oracle_status = oracles_read(storage).load(oracle.as_slice())?;

    if oracle_status.ending_round != ROUND_MAX {
        return ContractErr::OracleNotEnabled.std_err();
    }
    oracle_addresses(storage).update(|addresses| {
        Ok(addresses
            .into_iter()
            .filter(|addr| *addr != oracle)
            .collect())
    })?;
    oracles(storage).remove(oracle.as_slice());

    Ok(())
}

fn add_oracle<S: Storage>(
    storage: &mut S,
    oracle: CanonicalAddr,
    admin: CanonicalAddr,
) -> StdResult<()> {
    let oracle_status = oracles_read(storage)
        .load(oracle.as_slice())
        .unwrap_or_else(|_| OracleStatus::default());

    if oracle_status.ending_round == ROUND_MAX {
        return ContractErr::OracleNotEnabled.std_err();
    }
    if admin.is_empty() {
        return ContractErr::EmptyAdminAddr.std_err();
    }
    if !oracle_status.admin.is_empty() && oracle_status.admin != admin {
        return ContractErr::OverwritingAdmin.std_err();
    }

    let current_round = reporting_round_id_read(storage).load()?;
    let starting_round = if current_round != 0 && current_round == oracle_status.ending_round {
        current_round
    } else {
        current_round + 1
    };
    let index = oracle_addresses_read(storage).load()?.len() as u16;

    oracles(storage).save(
        oracle.as_slice(),
        &OracleStatus {
            starting_round,
            ending_round: ROUND_MAX,
            index,
            admin,
            ..oracle_status
        },
    )?;
    oracle_addresses(storage).update(|mut addreses| {
        addreses.push(oracle);
        Ok(addreses)
    })?;

    Ok(())
}

pub fn handle_transfer_admin<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    oracle: HumanAddr,
    new_admin: HumanAddr,
) -> StdResult<HandleResponse> {
    let oracle_addr = deps.api.canonical_address(&oracle)?;
    let sender_addr = deps.api.canonical_address(&env.message.sender)?;
    let new_admin_addr = deps.api.canonical_address(&new_admin)?;

    oracles(&mut deps.storage).update(oracle_addr.as_slice(), |status| {
        let mut status = status.unwrap_or_default();
        if status.admin != sender_addr {
            return ContractErr::NotAdmin.std_err();
        }
        status.pending_admin = Some(new_admin_addr);

        Ok(status)
    })?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "transfer admin"),
            log("oracle", oracle),
            log("sender", env.message.sender),
            log("new_admin", new_admin),
        ],
        data: None,
    })
}

pub fn handle_accept_admin<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    oracle: HumanAddr,
) -> StdResult<HandleResponse> {
    let oracle_addr = deps.api.canonical_address(&oracle)?;
    let sender_addr = deps.api.canonical_address(&env.message.sender)?;

    oracles(&mut deps.storage).update(oracle_addr.as_slice(), |status| {
        let mut status = status.unwrap_or_default();
        if let Some(pending_admin) = status.pending_admin {
            if pending_admin != sender_addr {
                return ContractErr::NotPendingAdmin.std_err();
            }
            status.admin = sender_addr;
            status.pending_admin = None;

            Ok(status)
        } else {
            ContractErr::PendingAdminMissing.std_err()
        }
    })?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("oracle_admin_updated", oracle),
            log("new_admin", env.message.sender),
        ],
        data: None,
    })
}

pub fn handle_request_new_round<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    let sender_addr = deps.api.canonical_address(&env.message.sender)?;
    let requester = requesters_read(&deps.storage)
        .may_load(sender_addr.as_slice())?
        .ok_or_else(|| ContractErr::Unauthorized.std())?;
    if !requester.authorized {
        return ContractErr::Unauthorized.std_err();
    }
    let current_round_id = reporting_round_id_read(&deps.storage).load()?;
    let current_round = rounds_read(&deps.storage).load(&current_round_id.to_be_bytes())?;
    if current_round.updated_at.is_none()
        && !timed_out(&deps.storage, current_round_id, env.block.time)?
    {
        return ContractErr::NotSupersedable.std_err();
    }

    let new_round_id = current_round_id + 1;
    if new_round_id <= requester.last_started_round + requester.delay
        && requester.last_started_round != 0
    {
        return ContractErr::DelayNotRespected.std_err();
    }

    initialize_new_round(&mut deps.storage, new_round_id, env.block.time)?;

    requesters(&mut deps.storage).save(
        sender_addr.as_slice(),
        &Requester {
            last_started_round: new_round_id,
            ..requester
        },
    )?;

    let round_id_serialized = to_binary(&new_round_id)?;
    Ok(HandleResponse {
        messages: vec![],
        log: vec![],
        data: Some(round_id_serialized),
    })
}

pub fn handle_withdraw_payment<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    oracle: HumanAddr,
    recipient: HumanAddr,
    amount: Uint128,
) -> StdResult<HandleResponse> {
    let sender = deps.api.canonical_address(&env.message.sender)?;
    let oracle = deps.api.canonical_address(&oracle)?;
    let oracle_status = oracles_read(&deps.storage).load(oracle.as_slice())?;

    if oracle_status.admin != sender {
        return ContractErr::NotAdmin.std_err();
    }
    if oracle_status.withdrawable < amount {
        return ContractErr::InsufficientWithdrawableFunds.std_err();
    }

    oracles(&mut deps.storage).save(
        oracle.as_slice(),
        &OracleStatus {
            withdrawable: (oracle_status.withdrawable - amount)?,
            ..oracle_status
        },
    )?;
    recorded_funds(&mut deps.storage).update(|mut funds| {
        funds.allocated = (funds.allocated - amount)?;
        Ok(funds)
    })?;

    let link = config_read(&deps.storage).load()?.link;
    let link_addr = deps.api.human_address(&link)?;

    let transfer_msg = WasmMsg::Execute {
        contract_addr: link_addr,
        msg: to_binary(&LinkMsg::Transfer { recipient, amount })?,
        send: vec![],
    };

    Ok(HandleResponse {
        messages: vec![transfer_msg.into()],
        log: vec![],
        data: None,
    })
}

pub fn handle_withdraw_funds<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    recipient: HumanAddr,
    amount: Uint128,
) -> StdResult<HandleResponse> {
    validate_ownership(deps, &env)?;

    let funds = recorded_funds_read(&deps.storage).load()?;
    let payment_amount = config_read(&deps.storage).load()?.payment_amount;
    let oracle_count = get_oracle_count(&deps)?;
    let available = (funds.available - required_reserve(payment_amount, oracle_count))?;

    if available < amount {
        return ContractErr::InsufficientReserveFunds.std_err();
    }

    let link = config_read(&deps.storage).load()?.link;
    let link_addr = deps.api.human_address(&link)?;

    let transfer_msg = WasmMsg::Execute {
        contract_addr: link_addr,
        msg: to_binary(&LinkMsg::Transfer { recipient, amount })?,
        send: vec![],
    };
    let update_funds_msg = WasmMsg::Execute {
        contract_addr: env.contract.address,
        msg: to_binary(&HandleMsg::UpdateAvailableFunds {})?,
        send: vec![],
    };

    Ok(HandleResponse {
        messages: vec![transfer_msg.into(), update_funds_msg.into()],
        log: vec![],
        data: None,
    })
}

pub fn handle_update_available_funds<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    let link_addr = config_read(&deps.storage).load()?.link;
    let query = QueryRequest::<()>::Wasm(WasmQuery::Smart {
        contract_addr: deps.api.human_address(&link_addr)?,
        msg: to_binary(&LinkQuery::Balance {
            address: env.contract.address,
        })?,
    });
    let prev_available: BalanceResponse = deps.querier.custom_query(&query)?;

    let funds = recorded_funds_read(&deps.storage).load()?;
    let now_available = (prev_available.balance - funds.allocated)?;

    if funds.available == now_available {
        return Ok(HandleResponse::default());
    }
    recorded_funds(&mut deps.storage).save(&Funds {
        available: now_available,
        allocated: funds.allocated,
    })?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "update_available_funds"),
            log("amount", now_available),
        ],
        data: None,
    })
}

pub fn handle_set_requester_permissions<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    requester: HumanAddr,
    authorized: bool,
    delay: u32,
) -> StdResult<HandleResponse> {
    validate_ownership(deps, &env)?;

    let requester_addr = deps.api.canonical_address(&requester)?;
    let requester_key = requester_addr.as_slice();
    let curr_requester = requesters_read(&deps.storage)
        .may_load(requester_key)?
        .unwrap_or_default();

    if curr_requester.authorized == authorized {
        return Ok(HandleResponse::default());
    }
    if authorized {
        requesters(&mut deps.storage).save(
            requester_key,
            &Requester {
                authorized,
                delay,
                ..curr_requester
            },
        )?;
    } else {
        requesters(&mut deps.storage).remove(requester_key);
    }

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "set_requester_permission"),
            log("requester", requester),
            log("authorized", authorized),
            log("delay", delay),
        ],
        data: None,
    })
}

pub fn handle_update_future_rounds<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    payment_amount: Uint128,
    min_submissions: u32,
    max_submissions: u32,
    restart_delay: u32,
    timeout: u32,
) -> StdResult<HandleResponse> {
    validate_ownership(deps, &env)?;

    let oracle_count = get_oracle_count(&deps)?;

    if min_submissions > max_submissions {
        return ContractErr::MinGreaterThanMax.std_err();
    }
    if (oracle_count as u32) < max_submissions {
        return ContractErr::MaxGreaterThanTotal.std_err();
    }
    if oracle_count != 0 && (oracle_count as u32) <= restart_delay {
        return ContractErr::DelayGreaterThanTotal.std_err();
    }
    let funds = recorded_funds_read(&deps.storage).load()?;
    if funds.available < required_reserve(payment_amount, oracle_count) {
        return ContractErr::InsufficientFunds.std_err();
    }
    if oracle_count > 0 && min_submissions == 0 {
        return ContractErr::MinLessThanZero.std_err();
    }

    config(&mut deps.storage).update(|state| {
        Ok(State {
            payment_amount,
            min_submission_count: min_submissions,
            max_submission_count: max_submissions,
            restart_delay,
            timeout,
            ..state
        })
    })?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "round_details_updated"),
            log("payment_amount", payment_amount),
            log("min_submissions", min_submissions),
            log("max_submissions", max_submissions),
            log("restart_delay", restart_delay),
            log("timeout", timeout),
        ],
        data: None,
    })
}

pub fn handle_set_validator<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    validator: HumanAddr,
) -> StdResult<HandleResponse> {
    validate_ownership(deps, &env)?;

    let validator_addr = deps.api.canonical_address(&validator)?;
    let old_validator = config_read(&deps.storage).load()?.validator;
    if old_validator == validator_addr {
        return Ok(HandleResponse::default());
    }

    config(&mut deps.storage).update(|state| {
        Ok(State {
            validator: validator_addr,
            ..state
        })
    })?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "validator_updated"),
            log("previous", deps.api.human_address(&old_validator)?),
            log("new", validator),
        ],
        data: None,
    })
}

pub fn handle_receive<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    env: Env,
    receive_msg: Cw20ReceiveMsg,
) -> StdResult<HandleResponse> {
    if receive_msg.msg.is_some() {
        return ContractErr::UnexpectedReceivePayload.std_err();
    }
    let msg = WasmMsg::Execute {
        contract_addr: env.contract.address,
        msg: to_binary(&HandleMsg::UpdateAvailableFunds {})?,
        send: vec![],
    };

    Ok(HandleResponse {
        messages: vec![msg.into()],
        log: vec![],
        data: None,
    })
}

pub fn handle_transfer_ownership<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    to: HumanAddr,
) -> StdResult<HandleResponse> {
    let to = deps.api.canonical_address(&to)?;
    owned::contract::handle_transfer_ownership(deps, env, to)
}

fn required_reserve(payment: Uint128, oracle_count: u8) -> Uint128 {
    Uint128(payment.u128() * oracle_count as u128 * RESERVE_ROUNDS)
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetAggregatorConfig {} => to_binary(&get_aggregator_config(deps)),
        QueryMsg::GetAllocatedFunds {} => to_binary(&get_allocated_funds(deps)),
        QueryMsg::GetAvailableFunds {} => to_binary(&get_available_funds(deps)),
        QueryMsg::GetWithdrawablePayment { oracle } => {
            to_binary(&get_withdrawable_payment(deps, oracle))
        }
        QueryMsg::GetOracleCount {} => to_binary(&get_oracle_count(deps)),
        QueryMsg::GetOracles {} => to_binary(&get_oracles(deps)),
        QueryMsg::GetAdmin { oracle } => to_binary(&get_admin(deps, oracle)),
        QueryMsg::GetRoundData { round_id } => to_binary(&get_round_data(deps, round_id)),
        QueryMsg::GetLatestRoundData {} => to_binary(&get_latest_round_data(deps)),
        QueryMsg::GetOracleRoundState {
            oracle,
            queried_round_id,
            timestamp,
        } => to_binary(&get_oracle_round_state(
            deps,
            oracle,
            queried_round_id,
            timestamp,
        )),
        QueryMsg::GetOwner {} => to_binary(&get_owner(deps)),
    }
}

pub fn get_aggregator_config<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ConfigResponse> {
    let config = config_read(&deps.storage).load()?;
    Ok(ConfigResponse {
        link: deps.api.human_address(&config.link)?,
        validator: deps.api.human_address(&config.validator)?,
        payment_amount: config.payment_amount,
        max_submission_count: config.min_submission_count,
        min_submission_count: config.max_submission_count,
        restart_delay: config.restart_delay,
        timeout: config.timeout,
        decimals: config.decimals,
        description: config.description,
        min_submission_value: config.min_submission_value,
        max_submission_value: config.max_submission_value,
    })
}

pub fn get_allocated_funds<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<Uint128> {
    Ok(recorded_funds_read(&deps.storage).load()?.allocated)
}

pub fn get_available_funds<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<Uint128> {
    Ok(recorded_funds_read(&deps.storage).load()?.available)
}

pub fn get_withdrawable_payment<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    oracle: HumanAddr,
) -> StdResult<Uint128> {
    let addr = deps.api.canonical_address(&oracle)?;
    let oracle = oracles_read(&deps.storage).load(addr.as_slice())?;
    Ok(oracle.withdrawable)
}

pub fn get_oracle_count<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<u8> {
    Ok(oracle_addresses_read(&deps.storage).load()?.len() as u8)
}

pub fn get_oracles<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<Vec<HumanAddr>> {
    let addresses = oracle_addresses_read(&deps.storage).load()?;
    let human_addresses = addresses
        .iter()
        .map(|addr| deps.api.human_address(addr))
        .collect::<StdResult<Vec<HumanAddr>>>()?;
    Ok(human_addresses)
}

pub fn get_admin<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    oracle: HumanAddr,
) -> StdResult<HumanAddr> {
    let addr = deps.api.canonical_address(&oracle)?;
    let oracle = oracles_read(&deps.storage).load(addr.as_slice())?;

    deps.api.human_address(&oracle.admin)
}

pub fn get_round_data<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    round_id: u32,
) -> StdResult<RoundDataResponse> {
    let round = rounds_read(&deps.storage).load(&round_id.to_be_bytes())?;
    if round.answered_in_round == 0 {
        return ContractErr::NoData.std_err();
    }
    Ok(RoundDataResponse {
        round_id,
        answer: round.answer,
        started_at: round.started_at,
        updated_at: round.updated_at,
        answered_in_round: round.answered_in_round,
    })
}

pub fn get_latest_round_data<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<RoundDataResponse> {
    let round_id = latest_round_id_read(&deps.storage).load()?;

    get_round_data(deps, round_id)
}

pub fn get_oracle_round_state<S: Storage, A: Api, Q: Querier>(
    _deps: &Extern<S, A, Q>,
    _oracle: HumanAddr,
    _queried_round_id: u32,
    _timestamp: u64, // Sending a timestamp from the client might not be a valid solution
) -> StdResult<OracleRoundStateResponse> {
    todo!()
    // let oracle_addr = deps.api.canonical_address(&oracle)?;

    // let oracle_count = get_oracle_count(deps)?;

    // if queried_round_id > 0 {
    //     let round_key = &queried_round_id.to_be_bytes();
    //     let round = rounds_read(&deps.storage).load(round_key)?;
    //     let round_details = details_read(&deps.storage).load(round_key)?;
    //     let oracle_status = oracles_read(&deps.storage).load(oracle_addr.as_slice())?;
    //     let available_funds = recorded_funds_read(&deps.storage).load()?.available;
    //     let State {
    //         restart_delay,
    //         payment_amount,
    //         ..
    //     } = config_read(&deps.storage).load()?;

    //     let is_elegible = if round.started_at.unwrap() > 0 {
    //         is_accepting_submissions(&deps.storage, queried_round_id)?
    //             && validate_oracle_round(&deps.storage, oracle_addr, queried_round_id, timestamp)
    //                 .is_ok()
    //     } else {
    //         let is_delayed = queried_round_id
    //             > oracle_status.last_started_round.unwrap() + restart_delay
    //             || oracle_status.last_started_round.unwrap() == 0;

    //         is_delayed
    //             && validate_oracle_round(&deps.storage, oracle_addr, queried_round_id, timestamp)
    //                 .is_ok()
    //     };
    //     let payment = if round.started_at.unwrap() > 0 {
    //         round_details.payment_amount
    //     } else {
    //         payment_amount
    //     };

    //     Ok(OracleRoundStateResponse {
    //         elegible_to_submit: is_elegible,
    //         round_id: queried_round_id,
    //         latest_submission: oracle_status.latest_submission,
    //         started_at: round.started_at.unwrap(),
    //         timeout: round_details.timeout,
    //         available_funds,
    //         oracle_count,
    //         payment_amount: payment,
    //     })
    // } else {
    //     unimplemented!()
    // }
}

fn validate_ownership<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    env: &Env,
) -> StdResult<()> {
    let owner = get_owner(deps)?;
    if env.message.sender != owner {
        return ContractErr::NotOwner.std_err();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prev_round_id() {
        assert_eq!(prev_round_id(1), Ok(0));
        assert_eq!(prev_round_id(0), Err(StdError::underflow(0, 1)));
    }
}
