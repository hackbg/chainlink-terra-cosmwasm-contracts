use cosmwasm_std::{
    attr, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, OverflowError,
    OverflowOperation, Response, StdError, StdResult, Storage, Timestamp, Uint128, WasmMsg,
};
use cw20::{BalanceResponse, Cw20ReceiveMsg};
use link_token::msg::{ExecuteMsg as LinkMsg, QueryMsg as LinkQuery};
use owned::contract::{get_owner, instantiate as owned_init};
use utils::median::calculate_median;

use crate::{error::*, msg::*, state::*};

static RESERVE_ROUNDS: u128 = 2;
static MAX_ORACLE_COUNT: u128 = 77;
static ROUND_MAX: u32 = u32::MAX;

pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    if msg.min_submission_value > msg.max_submission_value {
        return Err(ContractError::MinGreaterThanMax {});
    }
    ORACLE_ADDRESSES.save(deps.storage, &vec![])?;
    RECORDED_FUNDS.save(deps.storage, &Funds::default())?;
    REPORTING_ROUND_ID.save(deps.storage, &0)?;

    let link = deps.api.addr_validate(&msg.link)?;
    let validator = deps.api.addr_validate(&msg.validator)?;

    owned_init(
        deps.branch(),
        env.clone(),
        info,
        owned::msg::InstantiateMsg {},
    )?;

    CONFIG.save(
        deps.storage,
        &Config {
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
        },
    )?;

    ROUNDS.save(
        deps.storage,
        0.into(),
        &Round {
            answer: None,
            started_at: None,
            updated_at: None,
            answered_in_round: timestamp_to_seconds(env.block.time) as u32 - msg.timeout,
        },
    )?;
    LATEST_ROUND_ID.save(deps.storage, &0)?;

    Ok(Response::default())
}

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Submit {
            round_id,
            submission,
        } => execute_submit(deps, env, info, round_id, submission),
        ExecuteMsg::ChangeOracles {
            removed,
            added,
            added_admins,
            min_submissions,
            max_submissions,
            restart_delay,
        } => execute_change_oracles(
            deps,
            env,
            info,
            removed,
            added,
            added_admins,
            min_submissions,
            max_submissions,
            restart_delay,
        ),
        ExecuteMsg::WithdrawPayment {
            oracle,
            recipient,
            amount,
        } => execute_withdraw_payment(deps, env, info, oracle, recipient, amount),
        ExecuteMsg::WithdrawFunds { recipient, amount } => {
            execute_withdraw_funds(deps, env, info, recipient, amount)
        }
        ExecuteMsg::TransferAdmin { oracle, new_admin } => {
            execute_transfer_admin(deps, env, info, oracle, new_admin)
        }
        ExecuteMsg::AcceptAdmin { oracle } => execute_accept_admin(deps, env, info, oracle),
        ExecuteMsg::RequestNewRound {} => execute_request_new_round(deps, env, info),
        ExecuteMsg::SetRequesterPermissions {
            requester,
            authorized,
            delay,
        } => execute_set_requester_permissions(deps, env, info, requester, authorized, delay),
        ExecuteMsg::UpdateFutureRounds {
            payment_amount,
            min_submissions,
            max_submissions,
            restart_delay,
            timeout,
        } => execute_update_future_rounds(
            deps,
            env,
            info,
            payment_amount,
            min_submissions,
            max_submissions,
            restart_delay,
            timeout,
        ),
        ExecuteMsg::UpdateAvailableFunds {} => execute_update_available_funds(deps, env, info),
        ExecuteMsg::SetValidator { validator } => execute_set_validator(deps, env, info, validator),
        ExecuteMsg::Receive(receive_msg) => execute_receive(deps, env, info, receive_msg),
        ExecuteMsg::TransferOwnership { to } => execute_transfer_ownership(deps, env, info, to),
        ExecuteMsg::AcceptOwnership {} => execute_accept_ownership(deps, env, info),
    }
}

pub fn execute_submit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    round_id: u32,
    submission: Uint128,
) -> Result<Response, ContractError> {
    let Config {
        min_submission_value,
        max_submission_value,
        min_submission_count,
        max_submission_count,
        restart_delay,
        timeout,
        payment_amount,
        ..
    } = CONFIG.load(deps.storage)?;
    if submission < min_submission_value {
        return Err(ContractError::UnderMin {});
    }
    if submission > max_submission_value {
        return Err(ContractError::OverMax {});
    }
    let messages = vec![];
    let mut attributes = vec![];
    let timestamp = timestamp_to_seconds(env.block.time);

    let mut oracle = ORACLES.load(deps.storage, &info.sender)?;

    let rr_id = REPORTING_ROUND_ID.load(deps.storage)?;
    validate_oracle_round(deps.storage, &oracle, round_id, rr_id, timestamp)?;

    let mut round = ROUNDS
        .may_load(deps.storage, round_id.into())?
        .unwrap_or_default();
    let mut round_details: RoundDetails;

    // if new round and delay requirement is met
    if round_id == rr_id + 1
        && (oracle.last_started_round.is_none()
            || round_id > oracle.last_started_round.unwrap() + restart_delay)
    {
        // update round info if timed out
        let timed_out_round = prev_round_id(round_id)?;
        if timed_out(deps.storage, timed_out_round, timestamp).unwrap_or(false) {
            let prev_round_id = prev_round_id(timed_out_round)?;
            let prev_round = ROUNDS.load(deps.storage, prev_round_id.into())?;
            ROUNDS.update(
                deps.storage,
                timed_out_round.into(),
                |round| -> StdResult<_> {
                    Ok(Round {
                        answer: prev_round.answer,
                        answered_in_round: prev_round.answered_in_round,
                        updated_at: Some(timestamp),
                        ..round.unwrap()
                    })
                },
            )?;
            DETAILS.remove(deps.storage, timed_out_round.into());
        }
        REPORTING_ROUND_ID.save(deps.storage, &round_id)?;
        round_details = RoundDetails {
            submissions: vec![],
            max_submissions: max_submission_count,
            min_submissions: min_submission_count,
            timeout,
            payment_amount,
        };
        round.started_at = Some(timestamp);
        ROUNDS.save(deps.storage, round_id.into(), &round)?;
        attributes.extend_from_slice(&[
            attr("action", "new round"),
            attr("started by", &info.sender),
            attr("started at", timestamp),
        ]);

        oracle.last_started_round = Some(round_id);
    } else {
        round_details = DETAILS.load(deps.storage, round_id.into())?;
    }
    // record submission
    if !is_accepting_submissions(&round_details) {
        return Err(ContractError::NotAcceptingSubmissions {});
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
            calculate_median(&mut submissions).map_err(|_| ContractError::NoSubmissions {})?;
        ROUNDS.save(
            deps.storage,
            round_id.into(),
            &Round {
                answer: Some(Uint128(new_answer)),
                started_at: round.started_at,
                updated_at: Some(timestamp),
                answered_in_round: round_id,
            },
        )?;
        LATEST_ROUND_ID.save(deps.storage, &round_id)?;
        attributes.extend_from_slice(&[
            attr("action", "answer updated"),
            attr("current", Uint128(new_answer)),
            attr("round_id", round_id),
        ]);

        // TODO: send new value to validator
        // messages.push(FlaggingValidatorMsg::ValidateAnswer)
    }
    // pay oracle
    let payment = round_details.payment_amount;
    RECORDED_FUNDS.update(deps.storage, |funds| -> StdResult<_> {
        Ok(Funds {
            available: funds.available.checked_sub(payment)?,
            allocated: funds.allocated + payment,
        })
    })?;
    oracle.withdrawable += payment;

    ORACLES.save(deps.storage, &info.sender, &oracle)?;

    // save or delete round details
    if (round_details.submissions.len() as u32) < round_details.max_submissions {
        DETAILS.save(deps.storage, round_id.into(), &round_details)?;
    } else {
        DETAILS.remove(deps.storage, round_id.into());
    }

    Ok(Response {
        messages,
        submessages: vec![],
        attributes,
        data: None,
    })
}

fn validate_oracle_round(
    storage: &dyn Storage,
    oracle: &OracleStatus,
    round_id: u32,
    rr_id: u32,
    timestamp: u64,
) -> Result<(), ContractError> {
    if oracle.starting_round == 0 {
        return Err(ContractError::OracleNotEnabled {});
    }
    if oracle.starting_round > round_id {
        return Err(ContractError::OracleNotYetEnabled {});
    }
    if oracle.ending_round < round_id {
        return Err(ContractError::NoLongerAllowed {});
    }
    if oracle
        .last_reported_round
        .map_or(false, |id| id >= round_id)
    {
        return Err(ContractError::ReportingPreviousRound {});
    }
    let rr = ROUNDS.load(storage, rr_id.into())?;
    let unanswered = round_id + 1 == rr_id && rr.updated_at.is_none();
    if round_id != rr_id && round_id != rr_id + 1 && !unanswered {
        return Err(ContractError::InvalidRound {});
    }
    if round_id != 1 && !is_supersedable(storage, prev_round_id(round_id)?, timestamp)? {
        return Err(ContractError::NotSupersedable {});
    }
    Ok(())
}

fn is_supersedable(storage: &dyn Storage, round_id: u32, timestamp: u64) -> StdResult<bool> {
    let round = ROUNDS.load(storage, round_id.into())?;
    Ok(round.updated_at.unwrap() > 0 || timed_out(storage, round_id, timestamp)?)
}

fn timed_out(storage: &dyn Storage, round_id: u32, timestamp: u64) -> StdResult<bool> {
    let started_at = ROUNDS.load(storage, round_id.into())?.started_at;
    let round_details = DETAILS.may_load(storage, round_id.into())?;
    if let Some(round_details) = round_details {
        let timeout = round_details.timeout as u64;
        Ok(started_at.is_some() && timeout > 0 && started_at.unwrap() + timeout < timestamp)
    } else {
        Ok(false)
    }
}

fn initialize_new_round(storage: &mut dyn Storage, round_id: u32, timestamp: u64) -> StdResult<()> {
    // update round info if timed out
    let timed_out_round = prev_round_id(round_id)?;
    if timed_out(storage, timed_out_round, timestamp)? {
        let prev_round_id = prev_round_id(timed_out_round)?;
        let prev_round = ROUNDS.load(storage, prev_round_id.into())?;
        ROUNDS.update(storage, timed_out_round.into(), |round| -> StdResult<_> {
            Ok(Round {
                answer: prev_round.answer,
                answered_in_round: prev_round.answered_in_round,
                updated_at: Some(timestamp),
                ..round.unwrap()
            })
        })?;
        DETAILS.remove(storage, timed_out_round.into());
    }

    REPORTING_ROUND_ID.save(storage, &round_id)?;
    let Config {
        min_submission_count,
        max_submission_count,
        timeout,
        payment_amount,
        ..
    } = CONFIG.load(storage)?;
    DETAILS.save(
        storage,
        round_id.into(),
        &RoundDetails {
            submissions: vec![],
            max_submissions: max_submission_count,
            min_submissions: min_submission_count,
            timeout,
            payment_amount,
        },
    )?;
    ROUNDS.update(storage, round_id.into(), |round| -> StdResult<_> {
        Ok(Round {
            started_at: Some(timestamp),
            ..round.unwrap_or_default()
        })
    })?;

    Ok(())
}

fn prev_round_id(round_id: u32) -> StdResult<u32> {
    round_id.checked_sub(1).ok_or({
        StdError::overflow(OverflowError::new(
            OverflowOperation::Sub,
            round_id.to_string(),
            1.to_string(),
        ))
    })
}

fn timestamp_to_seconds(time: Timestamp) -> u64 {
    time.nanos() / 1_000_000_000
}

fn is_accepting_submissions(details: &RoundDetails) -> bool {
    details.max_submissions != 0
}

#[allow(clippy::too_many_arguments)]
pub fn execute_change_oracles(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    removed: Vec<String>,
    added: Vec<String>,
    added_admins: Vec<String>,
    min_submissions: u32,
    max_submissions: u32,
    restart_delay: u32,
) -> Result<Response, ContractError> {
    validate_ownership(deps.as_ref(), &info)?;

    let mut attributes = vec![];

    for oracle in removed {
        let oracle = deps.api.addr_validate(&oracle)?;
        remove_oracle(deps.storage, oracle)?;
    }

    if added.len() != added_admins.len() {
        return Err(ContractError::OracleAdminCountMismatch {});
    }
    let oracle_count: u128 = get_oracle_count(deps.as_ref(), env.clone())?.into();
    let new_count = oracle_count + added.len() as u128;
    if new_count > MAX_ORACLE_COUNT {
        return Err(ContractError::MaxOraclesAllowed {});
    }

    for (oracle, admin) in added.iter().zip(added_admins) {
        let oracle = deps.api.addr_validate(oracle)?;
        let admin = deps.api.addr_validate(&admin)?;
        add_oracle(deps.storage, oracle, admin)?;
    }

    let Config {
        payment_amount,
        timeout,
        ..
    } = CONFIG.load(deps.storage)?;

    let res = execute_update_future_rounds(
        deps,
        env,
        info,
        payment_amount,
        min_submissions,
        max_submissions,
        restart_delay,
        timeout,
    )?;
    attributes.extend_from_slice(&res.attributes);

    Ok(Response {
        messages: vec![],
        submessages: vec![],
        attributes, // TODO: add more logs
        data: None,
    })
}

fn remove_oracle(storage: &mut dyn Storage, oracle: Addr) -> Result<(), ContractError> {
    // TODO: is this needed?
    // let reporting_round = REPORTING_ROUND_ID_read.load()?;
    let oracle_status = ORACLES.load(storage, &oracle)?;

    if oracle_status.ending_round != ROUND_MAX {
        return Err(ContractError::OracleNotEnabled {});
    }
    ORACLE_ADDRESSES.update(storage, |addresses| -> StdResult<_> {
        Ok(addresses
            .into_iter()
            .filter(|addr| *addr != oracle)
            .collect())
    })?;
    ORACLES.remove(storage, &oracle);

    Ok(())
}

fn add_oracle(storage: &mut dyn Storage, oracle: Addr, admin: Addr) -> Result<(), ContractError> {
    let oracle_status = ORACLES
        .may_load(storage, &oracle)?
        .map(|oracle_status| {
            if oracle_status.ending_round == ROUND_MAX {
                return Err(ContractError::OracleNotEnabled {});
            }
            if oracle_status.admin != admin {
                return Err(ContractError::OverwritingAdmin {});
            }
            Ok(oracle_status)
        })
        .unwrap_or_else(|| {
            Ok(OracleStatus {
                withdrawable: Uint128::zero(),
                starting_round: 0,
                ending_round: 0,
                last_reported_round: None,
                last_started_round: None,
                latest_submission: None,
                index: 0,
                admin: admin.clone(),
                pending_admin: None,
            })
        })?;

    let current_round = REPORTING_ROUND_ID.load(storage)?;
    let starting_round = if current_round != 0 && current_round == oracle_status.ending_round {
        current_round
    } else {
        current_round + 1
    };

    let index = ORACLE_ADDRESSES.load(storage)?.len() as u16;

    ORACLES.save(
        storage,
        &oracle,
        &OracleStatus {
            starting_round,
            ending_round: ROUND_MAX,
            index,
            admin,
            ..oracle_status
        },
    )?;
    ORACLE_ADDRESSES.update(storage, |mut addreses| -> StdResult<_> {
        addreses.push(oracle);
        Ok(addreses)
    })?;

    Ok(())
}

pub fn execute_transfer_admin(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    oracle: String,
    new_admin: String,
) -> Result<Response, ContractError> {
    let oracle_addr = deps.api.addr_validate(&oracle)?;
    let new_admin_addr = deps.api.addr_validate(&new_admin)?;

    ORACLES.update(deps.storage, &oracle_addr, |status| {
        // let mut status = status.unwrap_or_default(); TODO
        let mut status = status.unwrap();
        if status.admin != info.sender {
            return Err(ContractError::NotAdmin {});
        }
        status.pending_admin = Some(new_admin_addr);

        Ok(status)
    })?;

    Ok(Response {
        messages: vec![],
        submessages: vec![],
        attributes: vec![
            attr("action", "transfer admin"),
            attr("oracle", oracle),
            attr("sender", info.sender),
            attr("new_admin", new_admin),
        ],
        data: None,
    })
}

pub fn execute_accept_admin(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    oracle: String,
) -> Result<Response, ContractError> {
    let oracle_addr = deps.api.addr_validate(&oracle)?;

    ORACLES.update(deps.storage, &oracle_addr, |status| {
        // let mut status = status.unwrap_or_default(); TODO
        let mut status = status.unwrap();
        if let Some(pending_admin) = status.pending_admin {
            if pending_admin != info.sender.clone() {
                return Err(ContractError::NotPendingAdmin {});
            }
            status.admin = info.sender.clone();
            status.pending_admin = None;

            Ok(status)
        } else {
            Err(ContractError::PendingAdminMissing {})
        }
    })?;

    Ok(Response {
        messages: vec![],
        submessages: vec![],
        attributes: vec![
            attr("oracle_admin_updated", oracle),
            attr("new_admin", info.sender),
        ],
        data: None,
    })
}

pub fn execute_request_new_round(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let requester = REQUESTERS
        .may_load(deps.storage, &info.sender)?
        .ok_or(ContractError::Unauthorized {})?;
    if !requester.authorized {
        return Err(ContractError::Unauthorized {});
    }
    let current_round_id = REPORTING_ROUND_ID.load(deps.storage)?;
    let current_round = ROUNDS.load(deps.storage, current_round_id.into())?;
    let timestamp = timestamp_to_seconds(env.block.time);
    if current_round.updated_at.is_none() && !timed_out(deps.storage, current_round_id, timestamp)?
    {
        return Err(ContractError::NotSupersedable {});
    }

    let new_round_id = current_round_id + 1;
    if new_round_id <= requester.last_started_round + requester.delay
        && requester.last_started_round != 0
    {
        return Err(ContractError::DelayNotRespected {});
    }

    initialize_new_round(deps.storage, new_round_id, timestamp)?;

    REQUESTERS.save(
        deps.storage,
        &info.sender,
        &Requester {
            last_started_round: new_round_id,
            ..requester
        },
    )?;

    let round_id_serialized = to_binary(&new_round_id)?;
    Ok(Response {
        messages: vec![],
        submessages: vec![],
        attributes: vec![],
        data: Some(round_id_serialized),
    })
}

pub fn execute_withdraw_payment(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    oracle: String,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let oracle = deps.api.addr_validate(&oracle)?;
    let oracle_status = ORACLES.load(deps.storage, &oracle)?;

    if oracle_status.admin != info.sender {
        return Err(ContractError::NotAdmin {});
    }
    if oracle_status.withdrawable < amount {
        return Err(ContractError::InsufficientWithdrawableFunds {});
    }

    ORACLES.save(
        deps.storage,
        &oracle,
        &OracleStatus {
            withdrawable: oracle_status
                .withdrawable
                .checked_sub(amount)
                .map_err(StdError::from)?,
            ..oracle_status
        },
    )?;
    RECORDED_FUNDS.update(deps.storage, |mut funds| -> StdResult<_> {
        funds.allocated = funds.allocated.checked_sub(amount)?;
        Ok(funds)
    })?;

    let link = CONFIG.load(deps.storage)?.link;

    let transfer_msg = WasmMsg::Execute {
        contract_addr: link.to_string(),
        msg: to_binary(&LinkMsg::Transfer { recipient, amount })?,
        send: vec![],
    };

    Ok(Response {
        messages: vec![transfer_msg.into()],
        submessages: vec![],
        attributes: vec![],
        data: None,
    })
}

pub fn execute_withdraw_funds(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    validate_ownership(deps.as_ref(), &info)?;

    let funds = RECORDED_FUNDS.load(deps.storage)?;
    let payment_amount = CONFIG.load(deps.storage)?.payment_amount;
    let oracle_count = get_oracle_count(deps.as_ref(), env.clone())?;
    let available = funds
        .available
        .checked_sub(required_reserve(payment_amount, oracle_count))
        .map_err(StdError::from)?;

    if available < amount {
        return Err(ContractError::InsufficientReserveFunds {});
    }

    let link = CONFIG.load(deps.storage)?.link;

    let transfer_msg = WasmMsg::Execute {
        contract_addr: link.to_string(),
        msg: to_binary(&LinkMsg::Transfer { recipient, amount })?,
        send: vec![],
    };
    let update_funds_msg = WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_binary(&ExecuteMsg::UpdateAvailableFunds {})?,
        send: vec![],
    };

    Ok(Response {
        messages: vec![transfer_msg.into(), update_funds_msg.into()],
        submessages: vec![],
        // TODO: assess if submessages would be an improvement here
        // submessages: vec![
        //     SubMsg {
        //         id: 0,
        //         msg: transfer_msg.into(),
        //         gas_limit: None,
        //         reply_on: ReplyOn::Always,
        //     },
        //     SubMsg {
        //         id: 1,
        //         msg: update_funds_msg.into(),
        //         gas_limit: None,
        //         reply_on: ReplyOn::Always,
        //     },
        // ],
        attributes: vec![],
        data: None,
    })
}

pub fn execute_update_available_funds(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    let link_addr = CONFIG.load(deps.storage)?.link;
    let prev_available: BalanceResponse = deps.querier.query_wasm_smart(
        link_addr,
        &LinkQuery::Balance {
            address: env.contract.address.to_string(),
        },
    )?;

    let funds = RECORDED_FUNDS.load(deps.storage)?;
    let now_available = prev_available
        .balance
        .checked_sub(funds.allocated)
        .map_err(StdError::from)?;

    if funds.available == now_available {
        return Ok(Response::default());
    }
    RECORDED_FUNDS.save(
        deps.storage,
        &Funds {
            available: now_available,
            allocated: funds.allocated,
        },
    )?;

    Ok(Response {
        messages: vec![],
        submessages: vec![],
        attributes: vec![
            attr("action", "update_available_funds"),
            attr("amount", now_available),
        ],
        data: None,
    })
}

pub fn execute_set_requester_permissions(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    requester: String,
    authorized: bool,
    delay: u32,
) -> Result<Response, ContractError> {
    validate_ownership(deps.as_ref(), &info)?;

    let requester_addr = deps.api.addr_validate(&requester)?;
    let curr_requester = REQUESTERS
        .may_load(deps.storage, &requester_addr)?
        .unwrap_or_default();

    if curr_requester.authorized == authorized {
        return Ok(Response::default());
    }
    if authorized {
        REQUESTERS.save(
            deps.storage,
            &requester_addr,
            &Requester {
                authorized,
                delay,
                ..curr_requester
            },
        )?;
    } else {
        REQUESTERS.remove(deps.storage, &requester_addr);
    }

    Ok(Response {
        messages: vec![],
        submessages: vec![],
        attributes: vec![
            attr("action", "set_requester_permission"),
            attr("requester", requester),
            attr("authorized", authorized),
            attr("delay", delay),
        ],
        data: None,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn execute_update_future_rounds(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    payment_amount: Uint128,
    min_submissions: u32,
    max_submissions: u32,
    restart_delay: u32,
    timeout: u32,
) -> Result<Response, ContractError> {
    validate_ownership(deps.as_ref(), &info)?;

    let oracle_count = get_oracle_count(deps.as_ref(), env)?;

    if min_submissions > max_submissions {
        return Err(ContractError::MinGreaterThanMax {});
    }
    if (oracle_count as u32) < max_submissions {
        return Err(ContractError::MaxGreaterThanTotal {});
    }
    if oracle_count != 0 && (oracle_count as u32) <= restart_delay {
        return Err(ContractError::DelayGreaterThanTotal {});
    }
    let funds = RECORDED_FUNDS.load(deps.storage)?;
    if funds.available < required_reserve(payment_amount, oracle_count) {
        return Err(ContractError::InsufficientFunds {});
    }
    if oracle_count > 0 && min_submissions == 0 {
        return Err(ContractError::MinLessThanZero {});
    }

    CONFIG.update(deps.storage, |config| -> StdResult<_> {
        Ok(Config {
            payment_amount,
            min_submission_count: min_submissions,
            max_submission_count: max_submissions,
            restart_delay,
            timeout,
            ..config
        })
    })?;

    Ok(Response {
        messages: vec![],
        submessages: vec![],
        attributes: vec![
            attr("action", "round_details_updated"),
            attr("payment_amount", payment_amount),
            attr("min_submissions", min_submissions),
            attr("max_submissions", max_submissions),
            attr("restart_delay", restart_delay),
            attr("timeout", timeout),
        ],
        data: None,
    })
}

pub fn execute_set_validator(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    validator: String,
) -> Result<Response, ContractError> {
    validate_ownership(deps.as_ref(), &info)?;

    let validator_addr = deps.api.addr_validate(&validator)?;
    let old_validator = CONFIG.load(deps.storage)?.validator;
    if old_validator == validator_addr {
        return Ok(Response::default());
    }

    CONFIG.update(deps.storage, |config| -> StdResult<_> {
        Ok(Config {
            validator: validator_addr,
            ..config
        })
    })?;

    Ok(Response {
        messages: vec![],
        submessages: vec![],
        attributes: vec![
            attr("action", "validator_updated"),
            attr("previous", old_validator.to_string()),
            attr("new", validator),
        ],
        data: None,
    })
}

pub fn execute_receive(
    _deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    receive_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    if !receive_msg.msg.is_empty() {
        return Err(ContractError::UnexpectedReceivePayload {});
    }
    let msg = WasmMsg::Execute {
        contract_addr: env.contract.address.to_string(),
        msg: to_binary(&ExecuteMsg::UpdateAvailableFunds {})?,
        send: vec![],
    };

    Ok(Response {
        messages: vec![msg.into()],
        submessages: vec![],
        attributes: vec![],
        data: None,
    })
}

pub fn execute_accept_ownership(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    owned::contract::execute_accept_ownership(deps, env, info).map_err(ContractError::from)
}

pub fn execute_transfer_ownership(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    to: String,
) -> Result<Response, ContractError> {
    let to = deps.api.addr_validate(&to)?;
    owned::contract::execute_transfer_ownership(deps, env, info, to).map_err(ContractError::from)
}

fn required_reserve(payment: Uint128, oracle_count: u8) -> Uint128 {
    Uint128(payment.u128() * oracle_count as u128 * RESERVE_ROUNDS)
}

pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetAggregatorConfig {} => to_binary(&get_aggregator_config(deps, env)?),
        QueryMsg::GetAllocatedFunds {} => to_binary(&get_allocated_funds(deps, env)?),
        QueryMsg::GetAvailableFunds {} => to_binary(&get_available_funds(deps, env)?),
        QueryMsg::GetWithdrawablePayment { oracle } => {
            to_binary(&get_withdrawable_payment(deps, env, oracle)?)
        }
        QueryMsg::GetOracleCount {} => to_binary(&get_oracle_count(deps, env)?),
        QueryMsg::GetOracles {} => to_binary(&get_oracles(deps, env)?),
        QueryMsg::GetAdmin { oracle } => to_binary(&get_admin(deps, env, oracle)?),
        QueryMsg::GetRoundData { round_id } => to_binary(&get_round_data(deps, env, round_id)?),
        QueryMsg::GetLatestRoundData {} => to_binary(&get_latest_round_data(deps, env)?),
        QueryMsg::GetOracleRoundState {
            oracle,
            queried_round_id,
        } => to_binary(&get_oracle_round_state(
            deps,
            env,
            oracle,
            queried_round_id,
        )?),
        QueryMsg::GetOwner {} => to_binary(&get_owner(deps)?),
    }
}

pub fn get_aggregator_config(deps: Deps, _env: Env) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        link: config.link,
        validator: config.validator,
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

pub fn get_allocated_funds(deps: Deps, _env: Env) -> StdResult<Uint128> {
    Ok(RECORDED_FUNDS.load(deps.storage)?.allocated)
}

pub fn get_available_funds(deps: Deps, _env: Env) -> StdResult<Uint128> {
    Ok(RECORDED_FUNDS.load(deps.storage)?.available)
}

pub fn get_withdrawable_payment(deps: Deps, _env: Env, oracle: String) -> StdResult<Uint128> {
    let addr = deps.api.addr_validate(&oracle)?;
    let oracle = ORACLES.load(deps.storage, &addr)?;
    Ok(oracle.withdrawable)
}

pub fn get_oracle_count(deps: Deps, _env: Env) -> StdResult<u8> {
    Ok(ORACLE_ADDRESSES.load(deps.storage)?.len() as u8)
}

pub fn get_oracles(deps: Deps, _env: Env) -> StdResult<Vec<Addr>> {
    ORACLE_ADDRESSES.load(deps.storage)
}

pub fn get_admin(deps: Deps, _env: Env, oracle: String) -> StdResult<Addr> {
    let addr = deps.api.addr_validate(&oracle)?;
    let oracle = ORACLES.load(deps.storage, &addr)?;
    Ok(oracle.admin)
}

pub fn get_round_data(deps: Deps, _env: Env, round_id: u32) -> StdResult<RoundDataResponse> {
    let round = ROUNDS.load(deps.storage, round_id.into())?;
    if round.answered_in_round == 0 {
        return Err(StdError::generic_err(ContractError::NoData {}.to_string()));
    }
    Ok(RoundDataResponse {
        round_id,
        answer: round.answer,
        started_at: round.started_at,
        updated_at: round.updated_at,
        answered_in_round: round.answered_in_round,
    })
}

pub fn get_latest_round_data(deps: Deps, env: Env) -> StdResult<RoundDataResponse> {
    let round_id = LATEST_ROUND_ID.load(deps.storage)?;

    get_round_data(deps, env, round_id)
}

pub fn get_oracle_round_state(
    _deps: Deps,
    _env: Env,
    _oracle: String,
    _queried_round_id: u32,
) -> StdResult<OracleRoundStateResponse> {
    // Implementation requires Env
    todo!()
}

fn validate_ownership(deps: Deps, info: &MessageInfo) -> Result<(), ContractError> {
    let owner = get_owner(deps)?;
    if info.sender != owner {
        return Err(ContractError::NotOwner {});
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{OverflowError, OverflowOperation};

    use super::*;

    #[test]
    fn test_prev_round_id() {
        assert_eq!(prev_round_id(1), Ok(0));
        assert_eq!(
            prev_round_id(0),
            Err(StdError::overflow(OverflowError::new(
                OverflowOperation::Sub,
                0.to_string(),
                1.to_string()
            )))
        );
    }
}
