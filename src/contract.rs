use crate::admin::{check_admin, has_administrator, read_administrator, write_administrator};
use crate::approval::{read_approval, read_approval_all, write_approval, write_approval_all};
use crate::balance::{
    check_minted, increment_supply, read_balance, read_supply, write_balance, write_minted,
};
use crate::event;
use crate::interface::{NonFungibleTokenTrait, WriteType};
use crate::metadata::{
     read_name, read_symbol, read_token_uri, write_name, write_symbol, write_token_uri,
};
use crate::owner::{check_owner, read_owner, write_owner, zero_address};
use crate::storage_types::DataKey;
use soroban_auth::verify;
use soroban_auth::{Identifier, Signature};
use soroban_sdk::{contractimpl, symbol, Bytes, Env};

pub struct NonFungibleToken;

fn read_nonce(env: &Env, id: &Identifier) -> i128 {
    let key = DataKey::Nonce(id.clone());
    env.storage().get(key).unwrap_or(Ok(0)).unwrap()
}

fn verify_and_consume_nonce(env: &Env, auth: &Signature, expected_nonce: i128) {
    match auth {
        Signature::Invoker => {
            if expected_nonce != 0 {
                panic!("nonce should be zero for Invoker")
            }
            return;
        }
        _ => {}
    }

    let id = auth.identifier(env);
    let key = DataKey::Nonce(id.clone());
    let nonce = read_nonce(env, &id);

    assert!(nonce == expected_nonce, "incorrect nonce");

    env.storage().set(key, &nonce + 1);
}

#[contractimpl]
impl NonFungibleTokenTrait for NonFungibleToken {
    fn initialize(env: Env, admin: Identifier, name: Bytes, symbol: Bytes) {
        assert!(!has_administrator(&env), "already initialized");

        write_administrator(&env, admin);
        write_name(&env, name);
        write_symbol(&env, symbol);
    }

    fn nonce(env: Env, id: Identifier) -> i128 {
        read_nonce(&env, &id)
    }

    fn admin(env: Env) -> Identifier {
        read_administrator(&env)
    }

    fn set_admin(env: Env, admin: Signature, nonce: i128, new_admin: Identifier) {
        check_admin(&env, &admin);

        verify_and_consume_nonce(&env, &admin, nonce);

        let admin_id = admin.identifier(&env);

        verify(
            &env,
            &admin,
            symbol!("set_admin"),
            (&admin_id, nonce, &new_admin),
        );
        write_administrator(&env, new_admin.clone());
        event::set_admin(&env, admin_id, new_admin);
    }

    fn name(env: Env) -> Bytes {
        read_name(&env)
    }

    fn symbol(env: Env) -> Bytes {
        read_symbol(&env)
    }

    fn token_uri(env: Env, id: i128) -> Bytes {
        read_token_uri(&env, id)
    }

    fn appr(env: Env, owner: Signature, nonce: i128, operator: Identifier, id: i128) {
        check_owner(&env, &owner.identifier(&env), id);
        verify_and_consume_nonce(&env, &owner, nonce);

        write_approval(&env, id, operator.clone());

        event::approve(&env, operator, id);
    }

    fn appr_all(env: Env, owner: Signature, nonce: i128, operator: Identifier, approved: bool) {
        verify_and_consume_nonce(&env, &owner, nonce);

        write_approval_all(&env, owner.identifier(&env), operator.clone(), approved);
        event::approve_all(&env, operator, owner.identifier(&env))
    }

    fn get_appr(env: Env, id: i128) -> Identifier {
        read_approval(&env, id)
    }

    fn is_appr(env: Env, owner: Identifier, operator: Identifier) -> bool {
        read_approval_all(&env, owner, operator)
    }

    fn balance(env: Env, owner: Identifier) -> i128 {
        read_balance(&env, owner)
    }

    fn owner(env: Env, id: i128) -> Identifier {
        read_owner(&env, id)
    }

    fn xfer(env: Env, from: Signature, nonce: i128, to: Identifier, id: i128) {
        check_owner(&env, &from.identifier(&env), id);
        verify_and_consume_nonce(&env, &from, nonce);

        write_owner(&env, id, to.clone());
        write_balance(&env, from.identifier(&env), WriteType::Remove);
        write_balance(&env, to.clone(), WriteType::Add);

        event::transfer(&env, from.identifier(&env), to, id);
    }

    fn xfer_from(
        env: Env,
        spender: Signature,
        from: Identifier,
        to: Identifier,
        nonce: i128,
        id: i128,
    ) {
        check_owner(&env, &from, id);
        verify_and_consume_nonce(&env, &spender, nonce);

        if spender.identifier(&env) == read_approval(&env, id)
            || read_approval_all(&env, from.clone(), spender.identifier(&env))
        {
            write_approval(&env, id, zero_address(&env));

            write_owner(&env, id, to.clone());
            write_balance(&env, from.clone(), WriteType::Remove);
            write_balance(&env, to.clone(), WriteType::Add);

            event::transfer(&env, from, to, id);
        } else {
            panic!("not approved")
        }
    }

    fn mint(env: Env, admin: Signature, nonce: i128, to: Identifier, id: i128, uri:Bytes ) {
        check_admin(&env, &admin);
        verify_and_consume_nonce(&env, &admin, nonce);

        write_balance(&env, to.clone(), WriteType::Add);
        write_owner(&env, id, to.clone());
        increment_supply(&env);
        write_token_uri(&env, id, uri);

        event::mint(&env, to, id)
    }

    fn mint_next(env: Env,uri:Bytes) {
        let to = Identifier::from(env.invoker());
        check_minted(&env, to.clone());
        write_minted(&env, to.clone());

        let next_id = read_supply(&env) + 1;

        write_balance(&env, to.clone(), WriteType::Add);
        write_owner(&env, next_id, to.clone());
        increment_supply(&env);
        write_token_uri(&env, next_id, uri);

        event::mint(&env, to, next_id)
    }

    fn burn(env: Env, admin: Signature, nonce: i128, id: i128) {
        check_admin(&env, &admin);
        verify_and_consume_nonce(&env, &admin, nonce);

        let from = read_owner(&env, id);
        write_owner(&env, id, zero_address(&env));
        write_balance(&env, from.clone(), WriteType::Remove);

        event::burn(&env, from, id);
    }
}
