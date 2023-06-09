/*!
Non-Fungible Token implementation with JSON serialization.
NOTES:
  - The maximum balance value is limited by U128 (2**128 - 1).
  - JSON calls should pass U128 as a base-10 string. E.g. "100".
  - The contract optimizes the inner trie structure by hashing account IDs. It will prevent some
    abuse of deep tries. Shouldn't be an issue, once NEAR clients implement full hashing of keys.
  - The contract tracks the change in storage before and after the call. If the storage increases,
    the contract requires the caller of the contract to attach enough deposit to the function call
    to cover the storage cost.
    This is done to prevent a denial of service attack on the contract by taking all available storage.
    If the storage decreases, the contract will issue a refund for the cost of the released storage.
    The unused tokens from the attached deposit are also refunded, so it's safe to
    attach more deposit than required.
  - To prevent the deployed contract from being modified or deleted, it should not have any access
    keys on its account.
*/
use near_contract_standards::non_fungible_token::metadata::{
    NFTContractMetadata, NonFungibleTokenMetadataProvider, TokenMetadata, NFT_METADATA_SPEC,
};
use near_contract_standards::non_fungible_token::NonFungibleToken;
use near_contract_standards::non_fungible_token::{Token, TokenId};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LazyOption;
use near_sdk::serde::Serialize;
use near_sdk::{
    env, log, near_bindgen, AccountId, BorshStorageKey, PanicOnDefault, Promise,
    PromiseOrValue,
};
#[derive(BorshDeserialize, BorshSerialize, Serialize)]
pub enum Status {
    All,
    Whitelist,
    None,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    tokens: NonFungibleToken,
    metadata: LazyOption<NFTContractMetadata>,
    mint_approval_status: Status,
    whitelist_accounts: Vec<AccountId>,
}

const DATA_IMAGE_SVG_NEAR_ICON: &str = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 288 288'%3E%3Cg id='l' data-name='l'%3E%3Cpath d='M187.58,79.81l-30.1,44.69a3.2,3.2,0,0,0,4.75,4.2L191.86,103a1.2,1.2,0,0,1,2,.91v80.46a1.2,1.2,0,0,1-2.12.77L102.18,77.93A15.35,15.35,0,0,0,90.47,72.5H87.34A15.34,15.34,0,0,0,72,87.84V201.16A15.34,15.34,0,0,0,87.34,216.5h0a15.35,15.35,0,0,0,13.08-7.31l30.1-44.69a3.2,3.2,0,0,0-4.75-4.2L96.14,186a1.2,1.2,0,0,1-2-.91V104.61a1.2,1.2,0,0,1,2.12-.77l89.55,107.23a15.35,15.35,0,0,0,11.71,5.43h3.13A15.34,15.34,0,0,0,216,201.16V87.84A15.34,15.34,0,0,0,200.66,72.5h0A15.35,15.35,0,0,0,187.58,79.81Z'/%3E%3C/g%3E%3C/svg%3E";

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    NonFungibleToken,
    Metadata,
    TokenMetadata,
    Enumeration,
    Approval,
}

#[near_bindgen]
impl Contract {
    /// Initializes the contract owned by `owner_id` with
    /// default metadata (for example purposes only).
    #[init]
    pub fn new_default_meta(owner_id: AccountId) -> Self {
        Self::new(
            owner_id,
            NFTContractMetadata {
                spec: NFT_METADATA_SPEC.to_string(),
                name: "QSTN NFT".to_string(),
                symbol: "QSTN".to_string(),
                icon: Some(DATA_IMAGE_SVG_NEAR_ICON.to_string()),
                base_uri: None,
                reference: None,
                reference_hash: None,
            },
        )
    }

    #[init]
    pub fn new(owner_id: AccountId, metadata: NFTContractMetadata) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        metadata.assert_valid();
        Self {
            tokens: NonFungibleToken::new(
                StorageKey::NonFungibleToken,
                owner_id,
                Some(StorageKey::TokenMetadata),
                Some(StorageKey::Enumeration),
                Some(StorageKey::Approval),
            ),
            metadata: LazyOption::new(StorageKey::Metadata, Some(&metadata)),
            mint_approval_status: Status::None,
            whitelist_accounts: vec![],
        }
    }

    /// Mint a new token with ID=`token_id` belonging to `receiver_id`.
    ///
    /// Since this example implements metadata, it also requires per-token metadata to be provided
    /// in this call. `self.tokens.mint` will also require it to be Some, since
    /// `StorageKey::TokenMetadata` was provided at initialization.
    ///
    /// `self.tokens.mint` will enforce `predecessor_account_id` to equal the `owner_id` given in
    /// initialization call to `new`.
    #[payable]
    pub fn nft_mint(
        &mut self,
        token_id: TokenId,
        receiver_id: AccountId,
        token_metadata: TokenMetadata,
    ) -> Token {
        // self.tokens.mint(token_id, receiver_id, Some(token_metadata))
        // owner can mint nft irrespective of mint_approval_status
        if env::current_account_id() == env::predecessor_account_id() {
            return self
                .tokens
                .internal_mint(token_id, receiver_id, Some(token_metadata));
        }
        match self.mint_approval_status {
            Status::All => {
                return self
                    .tokens
                    .internal_mint(token_id, receiver_id, Some(token_metadata));
            }
            Status::Whitelist => {
                assert!(
                    self.whitelist_accounts
                        .contains(&env::predecessor_account_id()),
                    "Only Whitelist Accounts can mint NFT"
                );
                return self
                    .tokens
                    .internal_mint(token_id, receiver_id, Some(token_metadata));
            }
            Status::None => {
                panic!("Minting is not allowed for Now")
            }
        }
    }

    #[payable]
    pub fn add_whitelist_account(&mut self, whitelist_account: AccountId) -> bool {
        //Checks only contract owner can add whitelist account
        assert!(
            env::current_account_id() == env::predecessor_account_id(),
            "Only Contract owner can add whitelist account"
        );
        self.whitelist_accounts.push(whitelist_account);
        return true;
    }

    #[payable]
    pub fn remove_whitelist_account(&mut self, whitelist_account: AccountId) -> bool {
        //Checks only contract owner can add whitelist account
        assert!(
            env::current_account_id() == env::predecessor_account_id(),
            "Only Contract owner can add whitelist account"
        );
        if let Some(index) = self.whitelist_accounts.iter().position(|x| x == &whitelist_account) {
            self.whitelist_accounts.remove(index);
        }
        return true;
    }

    pub fn get_whitelist_accounts(self) -> Vec<AccountId> {
        return self.whitelist_accounts;
    }

    #[payable]
    pub fn change_nft_approval_status(&mut self, approval_status: String) {
        //Checks only contract owner can change NFT Mint approval
        assert!(
            env::current_account_id() == env::predecessor_account_id(),
            "Only Contract owner can change NFT Mint approval"
        );
        match approval_status.as_str() {
            "all" => {
                log!("NFT approval status is set to ALL");
                self.mint_approval_status = Status::All
            }
            "whitelist" => {
                log!("NFT approval status is set to Whitelist ");
                self.mint_approval_status = Status::Whitelist
            }
            "none" => {
                log!("NFT approval status is set to Nne ");
                self.mint_approval_status = Status::None
            }
            _ => panic!("Invalid approval status: {}", approval_status),
        };
    }

    pub fn get_nft_approval_status(self) -> Status {
        return self.mint_approval_status;
    }

}

near_contract_standards::impl_non_fungible_token_core!(Contract, tokens);
near_contract_standards::impl_non_fungible_token_approval!(Contract, tokens);
near_contract_standards::impl_non_fungible_token_enumeration!(Contract, tokens);

#[near_bindgen]
impl NonFungibleTokenMetadataProvider for Contract {
    fn nft_metadata(&self) -> NFTContractMetadata {
        self.metadata.get().unwrap()
    }
}