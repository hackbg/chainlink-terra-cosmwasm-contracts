use std::convert::TryInto;

use chainlink_aggregator::{LatestAnswerResponse, QueryMsg::*, RoundDataResponse};
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, Event, MessageInfo, Order, Response, StdError,
    StdResult, Storage, Uint128,
};
use owned::contract::{
    execute_accept_ownership, execute_transfer_ownership, get_owner,
    instantiate as owned_instantiate,
};
use serde::de::DeserializeOwned;

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
            execute_transfer_ownership(deps, env, info, to.to_string()).map_err(ContractError::from)
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

    let mut response = Response::new();

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
            aggregator_addr: aggregator_addr.clone(),
        },
    )?;

    response = response.add_event(
        Event::new("confirm_aggregator").add_attribute("aggregator", aggregator_addr.to_string()),
    );

    Ok(response)
}

pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetPhaseAggregators {} => to_binary(&get_phase_aggregators(deps, env)?),
        QueryMsg::GetProposedRoundData { round_id } => {
            to_binary(&get_proposed_round_data(deps, env, round_id)?)
        }
        QueryMsg::GetProposedLatestRoundData {} => {
            to_binary(&get_proposed_latest_round_data(deps, env)?)
        }
        QueryMsg::GetProposedAggregator {} => to_binary(&get_proposed_aggregator(deps, env)?),
        QueryMsg::GetAggregator {} => to_binary(&get_aggregator(deps, env)?),
        QueryMsg::GetPhaseId {} => to_binary(&get_phase_id(deps, env)?),
        QueryMsg::GetOwner {} => to_binary(&get_owner(deps)?),
        QueryMsg::AggregatorQuery(GetRoundData { round_id }) => {
            to_binary(&get_round_data(deps, env, round_id)?)
        }
        QueryMsg::AggregatorQuery(GetLatestRoundData {}) => {
            to_binary(&get_latest_round_data(deps, env)?)
        }
        QueryMsg::AggregatorQuery(GetDecimals {}) => to_binary(&get_decimals(deps, env)?),
        QueryMsg::AggregatorQuery(GetVersion {}) => to_binary(&get_version(deps, env)?),
        QueryMsg::AggregatorQuery(GetDescription {}) => to_binary(&get_description(deps, env)?),
        QueryMsg::AggregatorQuery(GetLatestAnswer {}) => to_binary(&get_latest_answer(deps, env)?),
    }
}

pub fn get_decimals(deps: Deps, _env: Env) -> StdResult<u8> {
    query_current(deps, GetDecimals {})
}

pub fn get_version(deps: Deps, _env: Env) -> StdResult<Uint128> {
    query_current(deps, GetVersion {})
}

pub fn get_description(deps: Deps, _env: Env) -> StdResult<String> {
    query_current(deps, GetDescription {})
}

pub fn get_latest_answer(deps: Deps, _env: Env) -> StdResult<LatestAnswerResponse> {
    query_current(deps, GetLatestAnswer {})
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
        .query_wasm_smart(aggregator, &GetRoundData { round_id }.wrap())?;
    Ok(add_phase_ids(res, phase_id))
}

pub fn get_latest_round_data(deps: Deps, _env: Env) -> StdResult<RoundDataResponse> {
    let Phase {
        aggregator_addr,
        id,
    } = CURRENT_PHASE.load(deps.storage)?;
    let res: RoundDataResponse = deps
        .querier
        .query_wasm_smart(aggregator_addr, &GetLatestRoundData {}.wrap())?;
    Ok(add_phase_ids(res, id))
}

pub fn get_proposed_round_data(
    deps: Deps,
    _env: Env,
    round_id: u32,
) -> StdResult<RoundDataResponse> {
    let proposed = get_proposed(deps.storage)?;
    deps.querier
        .query_wasm_smart(proposed, &GetRoundData { round_id }.wrap())
}

pub fn get_proposed_latest_round_data(deps: Deps, _env: Env) -> StdResult<RoundDataResponse> {
    let proposed = get_proposed(deps.storage)?;
    deps.querier
        .query_wasm_smart(proposed, &GetLatestRoundData {}.wrap())
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

fn query_current<T: DeserializeOwned>(
    deps: Deps,
    query: chainlink_aggregator::QueryMsg,
) -> StdResult<T> {
    let Phase {
        aggregator_addr, ..
    } = CURRENT_PHASE.load(deps.storage)?;
    deps.querier
        .query_wasm_smart(aggregator_addr, &query.wrap())
}

fn add_phase(phase: u16, original_id: u32) -> u32 {
    (phase as u32)
        .checked_shl(PHASE_OFFSET.u128() as u32)
        .unwrap_or(0)
        | original_id
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
    use super::*;
    use cosmwasm_std::{
        testing::{mock_env, MockApi, MockStorage},
        Addr, Empty,
    };
    use cw_multi_test::{App, BankKeeper, Contract, ContractWrapper, Executor};

    const OWNER: &str = "admin0001";

    const PAYMENT_AMOUNT: Uint128 = Uint128::new(3);

    pub fn contract_proxy() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );
        Box::new(contract)
    }

    pub fn contract_flux() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            flux_aggregator::contract::execute,
            flux_aggregator::contract::instantiate,
            flux_aggregator::contract::query,
        );
        Box::new(contract)
    }

    pub fn contract_link_token() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            link_token::contract::execute,
            link_token::contract::instantiate,
            link_token::contract::query,
        );
        Box::new(contract)
    }
    pub fn contract_df_validator() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            deviation_flagging_validator::contract::execute,
            deviation_flagging_validator::contract::instantiate,
            deviation_flagging_validator::contract::query,
        );
        Box::new(contract)
    }

    pub fn instantiate_link(app: &mut App) -> Addr {
        let link_id = app.store_code(contract_link_token());
        app.instantiate_contract(
            link_id,
            Addr::unchecked(OWNER),
            &link_token::msg::InstantiateMsg {},
            &[],
            "LINK",
            None,
        )
        .unwrap()
    }

    pub fn instantiate_df_validator(app: &mut App) -> Addr {
        let df_validator_id = app.store_code(contract_df_validator());
        app.instantiate_contract(
            df_validator_id,
            Addr::unchecked(OWNER),
            &deviation_flagging_validator::msg::InstantiateMsg {
                flags: Addr::unchecked("flags").to_string(),
                flagging_threshold: 100000,
            },
            &[],
            "Deviation Flagging Validator",
            None,
        )
        .unwrap()
    }

    pub fn instantiate_flux(
        app: &mut App,
        link_addr: Addr,
        validator_addr: Addr,
        description: &str,
    ) -> Addr {
        let flux_aggregator_id = app.store_code(contract_flux());
        app.instantiate_contract(
            flux_aggregator_id,
            Addr::unchecked(OWNER),
            &flux_aggregator::msg::InstantiateMsg {
                link: link_addr.to_string(),
                payment_amount: PAYMENT_AMOUNT,
                timeout: 1800,
                validator: validator_addr.to_string(),
                min_submission_value: Uint128::new(1),
                max_submission_value: Uint128::new(10000000),
                decimals: 18,
                description: description.to_string(),
            },
            &[],
            "Flux aggregator",
            None,
        )
        .unwrap()
    }

    pub fn instantiate_proxy(app: &mut App, aggregator: Addr) -> Addr {
        let proxy_id = app.store_code(contract_proxy());
        let msg = crate::msg::InstantiateMsg {
            aggregator: aggregator.to_string(),
        };
        app.instantiate_contract(
            proxy_id,
            Addr::unchecked(OWNER),
            &msg,
            &[],
            "aggregator_proxy",
            None,
        )
        .unwrap()
    }

    fn mock_app() -> App {
        let env = mock_env();
        let api = MockApi::default();
        let bank = BankKeeper::new();

        App::new(api, env.block, bank, MockStorage::new())
    }

    #[test]
    fn instantiate_works() {
        let mut app = mock_app();
        let link_addr = instantiate_link(&mut app);
        let df_validator_addr = instantiate_df_validator(&mut app);
        let flux_aggregator_addr =
            instantiate_flux(&mut app, link_addr, df_validator_addr, "LINK/USD");
        let proxy_addr = instantiate_proxy(&mut app, flux_aggregator_addr);

        let phase_aggregators_query = QueryMsg::GetPhaseAggregators {};
        let res: PhaseAggregators = app
            .wrap()
            .query_wasm_smart(&proxy_addr, &phase_aggregators_query)
            .unwrap();

        assert_eq!(1, res.len());
    }

    #[test]
    fn propose_aggregator_works() {
        let mut app = mock_app();

        // first aggregator
        let link_addr = instantiate_link(&mut app);
        let df_validator_addr = instantiate_df_validator(&mut app);
        let flux_aggregator_addr =
            instantiate_flux(&mut app, link_addr, df_validator_addr, "LINK/USD");

        // second aggregator
        let link_addr = instantiate_link(&mut app);
        let df_validator_addr = instantiate_df_validator(&mut app);
        let flux_aggregator_addr2 =
            instantiate_flux(&mut app, link_addr, df_validator_addr, "LUNA/USD");

        let proxy_addr = instantiate_proxy(&mut app, flux_aggregator_addr);

        let phase_aggregators_query = QueryMsg::GetPhaseAggregators {};
        let res: PhaseAggregators = app
            .wrap()
            .query_wasm_smart(&proxy_addr, &phase_aggregators_query)
            .unwrap();

        assert_eq!(1, res.len());

        let propose_aggregator_msg = ExecuteMsg::ProposeAggregator {
            aggregator: flux_aggregator_addr2.to_string(),
        };
        app.execute_contract(
            Addr::unchecked(OWNER),
            proxy_addr.clone(),
            &propose_aggregator_msg,
            &[],
        )
        .unwrap();

        let proposed_aggregator_query = QueryMsg::GetProposedAggregator {};
        let res: Addr = app
            .wrap()
            .query_wasm_smart(&proxy_addr, &proposed_aggregator_query)
            .unwrap();

        assert_eq!(flux_aggregator_addr2.to_string(), res.to_string());
    }

    #[test]
    fn confirm_aggregator_works() {
        let mut app = mock_app();

        // first aggregator
        let link_addr = instantiate_link(&mut app);
        let df_validator_addr = instantiate_df_validator(&mut app);
        let flux_aggregator_addr =
            instantiate_flux(&mut app, link_addr, df_validator_addr, "LINK/USD");

        // second aggregator
        let link_addr = instantiate_link(&mut app);
        let df_validator_addr = instantiate_df_validator(&mut app);
        let flux_aggregator_addr2 =
            instantiate_flux(&mut app, link_addr, df_validator_addr, "LUNA/USD");

        let proxy_addr = instantiate_proxy(&mut app, flux_aggregator_addr);

        let phase_aggregators_query = QueryMsg::GetPhaseAggregators {};
        let res: PhaseAggregators = app
            .wrap()
            .query_wasm_smart(&proxy_addr, &phase_aggregators_query)
            .unwrap();

        assert_eq!(1, res.len());

        let propose_aggregator_msg = ExecuteMsg::ProposeAggregator {
            aggregator: flux_aggregator_addr2.to_string(),
        };
        app.execute_contract(
            Addr::unchecked(OWNER),
            proxy_addr.clone(),
            &propose_aggregator_msg,
            &[],
        )
        .unwrap();

        let proposed_aggregator_query = QueryMsg::GetProposedAggregator {};
        let res: Addr = app
            .wrap()
            .query_wasm_smart(&proxy_addr, &proposed_aggregator_query)
            .unwrap();

        assert_eq!(flux_aggregator_addr2.to_string(), res.to_string());

        let confirm_aggregator_msg = ExecuteMsg::ConfirmAggregator {
            aggregator: flux_aggregator_addr2.to_string(),
        };
        app.execute_contract(
            Addr::unchecked(OWNER),
            proxy_addr.clone(),
            &confirm_aggregator_msg,
            &[],
        )
        .unwrap();

        let res: PhaseAggregators = app
            .wrap()
            .query_wasm_smart(&proxy_addr, &phase_aggregators_query)
            .unwrap();

        assert_eq!(2, res.len());
    }

    #[test]
    fn querying_current_phase_aggregator_works() {
        let mut app = mock_app();

        // first aggregator
        let link_addr = instantiate_link(&mut app);
        let df_validator_addr = instantiate_df_validator(&mut app);
        let flux_aggregator_addr =
            instantiate_flux(&mut app, link_addr, df_validator_addr, "LINK/USD");

        // second aggregator
        let link_addr = instantiate_link(&mut app);
        let df_validator_addr = instantiate_df_validator(&mut app);
        let flux_aggregator_addr2 =
            instantiate_flux(&mut app, link_addr, df_validator_addr, "LUNA/USD");

        let proxy_addr = instantiate_proxy(&mut app, flux_aggregator_addr);

        let query_description = GetDescription {}.wrap();
        let res: String = app
            .wrap()
            .query_wasm_smart(&proxy_addr, &query_description)
            .unwrap();
        assert_eq!("LINK/USD".to_string(), res);

        let propose_aggregator_msg = ExecuteMsg::ProposeAggregator {
            aggregator: flux_aggregator_addr2.to_string(),
        };
        app.execute_contract(
            Addr::unchecked(OWNER),
            proxy_addr.clone(),
            &propose_aggregator_msg,
            &[],
        )
        .unwrap();

        let confirm_aggregator_msg = ExecuteMsg::ConfirmAggregator {
            aggregator: flux_aggregator_addr2.to_string(),
        };
        app.execute_contract(
            Addr::unchecked(OWNER),
            proxy_addr.clone(),
            &confirm_aggregator_msg,
            &[],
        )
        .unwrap();

        let res: String = app
            .wrap()
            .query_wasm_smart(&proxy_addr, &query_description)
            .unwrap();
        assert_eq!("LUNA/USD".to_string(), res);
    }
}
