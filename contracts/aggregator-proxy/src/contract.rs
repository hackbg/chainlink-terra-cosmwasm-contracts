use std::convert::TryInto;

use cosmwasm_std::{
    to_binary, Api, Binary, CanonicalAddr, Env, Extern, HandleResponse, HumanAddr, InitResponse,
    Order, Querier, QueryRequest, StdError, StdResult, Storage, Uint128, WasmQuery,
};
use flux_aggregator::msg::{ConfigResponse, QueryMsg as AggregatorQueryMsg, RoundDataResponse};
use owned::{
    contract::{get_owner, handle_accept_ownership, handle_transfer_ownership, init as owned_init},
    state::owner_read,
};

use crate::{
    msg::{HandleMsg, InitMsg, PhaseAggregators, QueryMsg},
    state::{
        current_phase, current_phase_read, get_phase_aggregator, phase_aggregators_read,
        proposed_aggregator, proposed_aggregator_read, set_phase_aggregator, Phase,
    },
};

static PHASE_OFFSET: Uint128 = Uint128(64);

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    owned_init(deps, env, owned::msg::InitMsg {})?;

    let aggregator_addr = deps.api.canonical_address(&msg.aggregator)?;

    set_phase_aggregator(&mut deps.storage, 1, &aggregator_addr)?;
    current_phase(&mut deps.storage).save(&Phase {
        id: 1,
        aggregator_addr,
    })?;

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::ProposeAggregator { aggregator } => {
            handle_propose_aggregator(deps, env, aggregator)
        }
        HandleMsg::ConfirmAggregator { aggregator } => {
            handle_confirm_aggregator(deps, env, aggregator)
        }
        HandleMsg::TransferOwnership { to } => handle_transfer_ownership(deps, env, to),
        HandleMsg::AcceptOwnership {} => handle_accept_ownership(deps, env),
    }
}

pub fn handle_propose_aggregator<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    aggregator: HumanAddr,
) -> StdResult<HandleResponse> {
    let sender = deps.api.canonical_address(&env.message.sender)?;
    if sender != owner_read(&deps.storage).load()?.owner {
        return Err(StdError::generic_err("Not owner"));
    }

    let aggregator_addr = deps.api.canonical_address(&aggregator)?;
    proposed_aggregator(&mut deps.storage).save(&aggregator_addr)?;

    Ok(HandleResponse::default())
}

pub fn handle_confirm_aggregator<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    aggregator: HumanAddr,
) -> StdResult<HandleResponse> {
    let sender = deps.api.canonical_address(&env.message.sender)?;
    if sender != owner_read(&deps.storage).load()?.owner {
        return Err(StdError::generic_err("Not owner"));
    }
    let aggregator_addr = deps.api.canonical_address(&aggregator)?;
    let proposed = proposed_aggregator_read(&deps.storage)
        .may_load()?
        .ok_or_else(|| StdError::generic_err("Invalid proposed aggregator"))?;
    if proposed != aggregator_addr {
        return Err(StdError::generic_err("Invalid proposed aggregator"));
    }

    proposed_aggregator(&mut deps.storage).remove();

    let phase = current_phase(&mut deps.storage).load()?;
    let new_id = phase.id + 1;
    set_phase_aggregator(&mut deps.storage, new_id, &aggregator_addr)?;
    current_phase(&mut deps.storage).save(&Phase {
        id: new_id,
        aggregator_addr,
    })?;

    Ok(HandleResponse::default())
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetPhaseAggregators {} => to_binary(&get_phase_aggregators(deps)),
        QueryMsg::GetRoundData { round_id } => to_binary(&get_round_data(deps, round_id)),
        QueryMsg::GetLatestRoundData {} => to_binary(&get_latest_round_data(deps)),
        QueryMsg::GetProposedRoundData { round_id } => {
            to_binary(&get_proposed_round_data(deps, round_id))
        }
        QueryMsg::GetProposedLatestRoundData {} => to_binary(&get_proposed_latest_round_data(deps)),
        QueryMsg::GetProposedAggregator {} => to_binary(&get_proposed_aggregator(deps)),
        QueryMsg::GetAggregator {} => to_binary(&get_aggregator(deps)),
        QueryMsg::GetPhaseId {} => to_binary(&get_phase_id(deps)),
        QueryMsg::GetDecimals {} => to_binary(&get_decimals(deps)),
        QueryMsg::GetDescription {} => to_binary(&get_description(deps)),
        QueryMsg::GetOwner {} => to_binary(&get_owner(deps)),
    }
}

macro_rules! query {
    ($deps:ident, $addr:ident, $query_type:ident $($prop_val:ident), * => $ret:ty) => {{
        let query = QueryRequest::<()>::Wasm(WasmQuery::Smart {
            contract_addr: $deps.api.human_address(&$addr)?,
            msg: to_binary(&AggregatorQueryMsg::$query_type {
                $(
                    $prop_val,
                )*
            })?,
        });
        let res: StdResult<$ret> = $deps.querier.custom_query(&query)?;
        res
    }};
}

pub fn get_phase_aggregators<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<PhaseAggregators> {
    phase_aggregators_read(&deps.storage)
        .range(None, None, Order::Ascending)
        .map(|entry| {
            entry.and_then(|aggregator| {
                Ok((
                    u16::from_be_bytes(aggregator.0.as_slice().try_into().unwrap()),
                    deps.api.human_address(&aggregator.1)?,
                ))
            })
        })
        .collect()
}

pub fn get_round_data<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    round_id: u32,
) -> StdResult<RoundDataResponse> {
    let phase_id = (round_id >> PHASE_OFFSET.u128())
        .try_into()
        .map_err(|_| StdError::generic_err("Failed parse"))?;
    let aggregator = get_phase_aggregator(&deps.storage, phase_id)?;
    let res = query!(deps, aggregator, GetRoundData round_id => RoundDataResponse)?;
    Ok(add_phase_ids(res, phase_id))
}

pub fn get_latest_round_data<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<RoundDataResponse> {
    let Phase {
        aggregator_addr,
        id,
    } = current_phase_read(&deps.storage).load()?;
    let res = query!(deps, aggregator_addr, GetLatestRoundData => RoundDataResponse)?;
    Ok(add_phase_ids(res, id))
}

pub fn get_proposed_round_data<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    round_id: u32,
) -> StdResult<RoundDataResponse> {
    let proposed = get_proposed(&deps.storage)?;
    query!(deps, proposed, GetRoundData round_id => RoundDataResponse)
}

pub fn get_proposed_latest_round_data<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<RoundDataResponse> {
    let proposed = get_proposed(&deps.storage)?;
    query!(deps, proposed, GetLatestRoundData => RoundDataResponse)
}

pub fn get_proposed_aggregator<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<HumanAddr> {
    proposed_aggregator_read(&deps.storage)
        .load()
        .and_then(|aggregator_addr| deps.api.human_address(&aggregator_addr))
}

pub fn get_aggregator<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<HumanAddr> {
    current_phase_read(&deps.storage)
        .load()
        .and_then(|phase| deps.api.human_address(&phase.aggregator_addr))
}

pub fn get_phase_id<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<u16> {
    Ok(current_phase_read(&deps.storage).load()?.id)
}

pub fn get_decimals<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<u8> {
    let aggregator_addr = current_phase_read(&deps.storage).load()?.aggregator_addr;
    let res = query!(deps, aggregator_addr, GetAggregatorConfig => ConfigResponse)?;
    Ok(res.decimals)
}

pub fn get_description<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<String> {
    let aggregator_addr = current_phase_read(&deps.storage).load()?.aggregator_addr;
    let res = query!(deps, aggregator_addr, GetAggregatorConfig => ConfigResponse)?;
    Ok(res.description)
}

fn get_proposed<S: Storage>(storage: &S) -> StdResult<CanonicalAddr> {
    proposed_aggregator_read(storage)
        .may_load()?
        .ok_or_else(|| StdError::generic_err("No proposed aggregator present"))
}

fn add_phase_ids(round_data: RoundDataResponse, phase_id: u16) -> RoundDataResponse {
    RoundDataResponse {
        round_id: add_phase(phase_id, round_data.round_id),
        answer: round_data.answer,
        started_at: round_data.started_at,
        updated_at: round_data.updated_at,
        answered_in_round: add_phase(phase_id, round_data.answered_in_round),
    }
}

fn add_phase(phase: u16, original_id: u32) -> u32 {
    (phase as u32) << PHASE_OFFSET.u128() | original_id
}

// #[cfg(test)]
// mod tests {
//     use super::*;
// }
