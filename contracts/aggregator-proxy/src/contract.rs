use cosmwasm_std::{
    to_binary, Api, Binary, CanonicalAddr, Env, Extern, HandleResponse, HumanAddr, InitResponse,
    Querier, StdError, StdResult, Storage, Uint128,
};
use owned::{
    contract::{get_owner, handle_accept_ownership, handle_transfer_ownership, init as owned_init},
    state::owner_read,
};

use crate::{
    msg::{HandleMsg, InitMsg, QueryMsg},
    state::{
        current_phase, current_phase_read, phase_aggregators, proposed_aggregator,
        proposed_aggregator_read, set_phase_aggregator, Phase,
    },
};

static PHASE_OFFSET: Uint128 = Uint128(64);
static PHASE_SIZE: Uint128 = Uint128(16);
static MAX_ID: Uint128 = Uint128(2_u128.pow(80) - 1);

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
        .ok_or(StdError::generic_err("Invalid proposed aggregator"))?;
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
        QueryMsg::GetRoundData { round_id } => todo!(),
        QueryMsg::GetLatestRoundData {} => todo!(),
        QueryMsg::GetProposedRoundData { round_id } => todo!(),
        QueryMsg::GetProposedLatestRoundData {} => todo!(),
        QueryMsg::GetProposedAggregator {} => todo!(),
        QueryMsg::GetPhase {} => todo!(),
        QueryMsg::GetDecimals {} => todo!(),
        QueryMsg::GetDescription {} => todo!(),
        QueryMsg::GetOwner {} => to_binary(&get_owner(deps)),
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
// }
