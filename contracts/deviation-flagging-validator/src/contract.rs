use cosmwasm_std::{
    to_binary, Api, Binary, CanonicalAddr, Env, Extern, HandleResponse, HumanAddr, InitResponse,
    Querier, StdError, StdResult, Storage, Uint128,
};

use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{flags, flags_read, State};

static THRESHOLD_MULTIPLIER: u32 = 100000;

// TODO should probably use type-safe wrappers
pub enum FlagInterface {
    GetFlag(CanonicalAddr),
    GetFlags(Vec<CanonicalAddr>),
    RaiseFlag(CanonicalAddr),
    RaiseFlags(Vec<CanonicalAddr>),
    LowerFlags(Vec<CanonicalAddr>),
    SetRaisingAccessController(CanonicalAddr),
}

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let flags_addr = deps.api.canonical_address(&msg.flags)?;

    let state = State {
        flags: flags_addr,
        flagging_treshold: msg.flagging_threshold,
    };

    flags(&mut deps.storage).save(&state)?;

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::SetFlagsAddress { flags } => todo!(),
        HandleMsg::SetFlaggingTreshold { treshold } => todo!(),
    }
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Validate {} => todo!(),
        QueryMsg::IsValid {} => todo!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, from_binary, StdError};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            flags: HumanAddr::from("flags"),
            flagging_threshold: 100000,
        };
        let env = mock_env("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }
}
