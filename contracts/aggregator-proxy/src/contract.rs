use std::convert::TryInto;

use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdError, StdResult,
    Storage, Uint128,
};
use flux_aggregator::msg::{ConfigResponse, QueryMsg as AggregatorQueryMsg, RoundDataResponse};
use owned::contract::{
    execute_accept_ownership, execute_transfer_ownership, get_owner,
    instantiate as owned_instantiate,
};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, PhaseAggregators, QueryMsg},
    state::{Phase, CURRENT_PHASE, PHASE_AGGREGATORS, PROPOSED_AGGREGATOR},
};

static PHASE_OFFSET: Uint128 = Uint128::new(64);

pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    owned_instantiate(deps.branch(), env, info, owned::msg::InstantiateMsg {})?;

    let aggregator_addr = deps.api.addr_validate(&msg.aggregator)?;

    PHASE_AGGREGATORS.save(deps.storage, 1.into(), &aggregator_addr)?;
    CURRENT_PHASE.save(
        deps.storage,
        &Phase {
            id: 1,
            aggregator_addr,
        },
    )?;

    Ok(Response::default())
}

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ProposeAggregator { aggregator } => {
            execute_propose_aggregator(deps, env, info, aggregator)
        }
        ExecuteMsg::ConfirmAggregator { aggregator } => {
            execute_confirm_aggregator(deps, env, info, aggregator)
        }
        ExecuteMsg::TransferOwnership { to } => {
            execute_transfer_ownership(deps, env, info, to).map_err(ContractError::from)
        }
        ExecuteMsg::AcceptOwnership {} => {
            execute_accept_ownership(deps, env, info).map_err(ContractError::from)
        }
    }
}

pub fn execute_propose_aggregator(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    aggregator: String,
) -> Result<Response, ContractError> {
    validate_ownership(deps.as_ref(), &info)?;

    let aggregator_addr = deps.api.addr_validate(&aggregator)?;
    PROPOSED_AGGREGATOR.save(deps.storage, &aggregator_addr)?;

    Ok(Response::default())
}

pub fn execute_confirm_aggregator(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    aggregator: String,
) -> Result<Response, ContractError> {
    validate_ownership(deps.as_ref(), &info)?;

    let aggregator_addr = deps.api.addr_validate(&aggregator)?;

    let proposed = PROPOSED_AGGREGATOR
        .may_load(deps.storage)?
        .ok_or(ContractError::InvalidProposedAggregator {})?;
    if proposed != aggregator_addr {
        return Err(ContractError::InvalidProposedAggregator {});
    }

    PROPOSED_AGGREGATOR.remove(deps.storage);

    let phase = CURRENT_PHASE.load(deps.storage)?;
    let new_id = phase.id + 1;
    PHASE_AGGREGATORS.save(deps.storage, new_id.into(), &aggregator_addr)?;
    CURRENT_PHASE.save(
        deps.storage,
        &Phase {
            id: new_id,
            aggregator_addr,
        },
    )?;

    Ok(Response::default())
}

pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetPhaseAggregators {} => to_binary(&get_phase_aggregators(deps, env)?),
        QueryMsg::GetRoundData { round_id } => to_binary(&get_round_data(deps, env, round_id)?),
        QueryMsg::GetLatestRoundData {} => to_binary(&get_latest_round_data(deps, env)?),
        QueryMsg::GetProposedRoundData { round_id } => {
            to_binary(&get_proposed_round_data(deps, env, round_id)?)
        }
        QueryMsg::GetProposedLatestRoundData {} => {
            to_binary(&get_proposed_latest_round_data(deps, env)?)
        }
        QueryMsg::GetProposedAggregator {} => to_binary(&get_proposed_aggregator(deps, env)?),
        QueryMsg::GetAggregator {} => to_binary(&get_aggregator(deps, env)?),
        QueryMsg::GetPhaseId {} => to_binary(&get_phase_id(deps, env)?),
        QueryMsg::GetDecimals {} => to_binary(&get_decimals(deps, env)?),
        QueryMsg::GetDescription {} => to_binary(&get_description(deps, env)?),
        QueryMsg::GetOwner {} => to_binary(&get_owner(deps)?),
    }
}

pub fn get_phase_aggregators(deps: Deps, _env: Env) -> StdResult<PhaseAggregators> {
    PHASE_AGGREGATORS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|entry| {
            entry.map(|aggregator| {
                (
                    u16::from_be_bytes(aggregator.0.as_slice().try_into().unwrap()),
                    aggregator.1,
                )
            })
        })
        .collect()
}

pub fn get_round_data(deps: Deps, _env: Env, round_id: u32) -> StdResult<RoundDataResponse> {
    let phase_id: u16 = (round_id >> PHASE_OFFSET.u128())
        .try_into()
        // TODO improve error
        .map_err(|_| StdError::generic_err("Failed parse"))?;
    let aggregator = PHASE_AGGREGATORS.load(deps.storage, phase_id.into())?;
    let res: RoundDataResponse = deps
        .querier
        .query_wasm_smart(aggregator, &AggregatorQueryMsg::GetRoundData { round_id })?;
    Ok(add_phase_ids(res, phase_id))
}

pub fn get_latest_round_data(deps: Deps, _env: Env) -> StdResult<RoundDataResponse> {
    let Phase {
        aggregator_addr,
        id,
    } = CURRENT_PHASE.load(deps.storage)?;
    let res: RoundDataResponse = deps
        .querier
        .query_wasm_smart(aggregator_addr, &AggregatorQueryMsg::GetLatestRoundData {})?;
    Ok(add_phase_ids(res, id))
}

pub fn get_proposed_round_data(
    deps: Deps,
    _env: Env,
    round_id: u32,
) -> StdResult<RoundDataResponse> {
    let proposed = get_proposed(deps.storage)?;
    deps.querier
        .query_wasm_smart(proposed, &AggregatorQueryMsg::GetRoundData { round_id })
}

pub fn get_proposed_latest_round_data(deps: Deps, _env: Env) -> StdResult<RoundDataResponse> {
    let proposed = get_proposed(deps.storage)?;
    deps.querier
        .query_wasm_smart(proposed, &AggregatorQueryMsg::GetLatestRoundData {})
}

pub fn get_proposed_aggregator(deps: Deps, _env: Env) -> StdResult<Addr> {
    PROPOSED_AGGREGATOR.load(deps.storage)
}

pub fn get_aggregator(deps: Deps, _env: Env) -> StdResult<Addr> {
    CURRENT_PHASE
        .load(deps.storage)
        .map(|phase| phase.aggregator_addr)
}

pub fn get_phase_id(deps: Deps, _env: Env) -> StdResult<u16> {
    Ok(CURRENT_PHASE.load(deps.storage)?.id)
}

pub fn get_decimals(deps: Deps, _env: Env) -> StdResult<u8> {
    let aggregator_addr = CURRENT_PHASE.load(deps.storage)?.aggregator_addr;
    let res: ConfigResponse = deps
        .querier
        .query_wasm_smart(aggregator_addr, &AggregatorQueryMsg::GetAggregatorConfig {})?;

    Ok(res.decimals)
}

pub fn get_description(deps: Deps, _env: Env) -> StdResult<String> {
    let aggregator_addr = CURRENT_PHASE.load(deps.storage)?.aggregator_addr;
    let res: ConfigResponse = deps
        .querier
        .query_wasm_smart(aggregator_addr, &AggregatorQueryMsg::GetAggregatorConfig {})?;

    Ok(res.description)
}

fn get_proposed(storage: &dyn Storage) -> StdResult<Addr> {
    PROPOSED_AGGREGATOR
        .may_load(storage)?
        .ok_or(ContractError::NoProposedAggregator {})
        .map_err(|err| StdError::generic_err(err.to_string()))
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

fn validate_ownership(deps: Deps, info: &MessageInfo) -> Result<(), ContractError> {
    let owner = get_owner(deps)?;
    if info.sender != owner {
        return Err(ContractError::NotOwner {});
    }
    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use super::*;
// }
