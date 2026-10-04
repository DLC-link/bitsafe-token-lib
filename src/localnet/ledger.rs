//! A small client for the sandbox's JSON ledger API. canton-lib's `ledger`
//! crate cannot create contracts or allocate parties, so those calls go
//! through `canton-api-client`.

use canton_api_client::{
    apis::{Error, configuration::Configuration, default_api},
    models::{
        AllocatePartyRequest, Command, CommandOneOf1, CommandOneOf3, CreateCommand,
        DisclosedContract, ExerciseCommand, JsActiveContract, JsCommands,
        JsSubmitAndWaitForTransactionRequest, JsSubmitAndWaitForTransactionResponse,
    },
};
use serde_json::Value;

use crate::kits::{bitsafe_api::ContractInfo, canton};

/// The user every submission names. The sandbox runs without auth, so no
/// token carries a user id, and the ledger rejects a submission without one.
/// The ledger accepts any user id here; the user need not exist.
pub(crate) const USER_ID: &str = "localnet-suite";

/// The sandbox's participant1, as the suite talks to it.
pub(crate) struct Ledger {
    pub(crate) host: String,
    pub(crate) token: String,
    config: Configuration,
}

impl Ledger {
    /// The ledger at `LOCALNET_LEDGER_HOST`, by default `http://localhost:7575`.
    ///
    /// The sandbox checks no token. canton-lib still sends a bearer header,
    /// and the ledger rejects an empty one, so the token is a placeholder.
    pub(crate) fn from_env() -> Ledger {
        let host = std::env::var("LOCALNET_LEDGER_HOST")
            .unwrap_or_else(|_| "http://localhost:7575".to_string());
        let token = "localnet".to_string();
        let config = Configuration {
            base_path: host.clone(),
            bearer_access_token: Some(token.clone()),
            ..Configuration::default()
        };
        Ledger {
            host,
            token,
            config,
        }
    }

    /// The id of the existing party whose id starts with `<hint>::`.
    pub(crate) async fn party(&self, hint: &str) -> Result<String, String> {
        let parties = default_api::get_v2_parties(&self.config, None, None, None, None)
            .await
            .map_err(|e| format!("get_v2_parties failed: {}", describe(e)))?;
        let prefix = format!("{}::", hint);
        parties
            .party_details
            .into_iter()
            .map(|details| details.party)
            .find(|party| party.starts_with(&prefix))
            .ok_or_else(|| format!("No party {} on the ledger", prefix))
    }

    /// Allocates a fresh party `<hint>-<8 hex chars>` and returns its id.
    pub(crate) async fn allocate_party(&self, hint: &str) -> Result<String, String> {
        let suffix = uuid::Uuid::new_v4().simple().to_string();
        let request = AllocatePartyRequest {
            party_id_hint: Some(format!("{}-{}", hint, &suffix[..8])),
            ..Default::default()
        };
        let response = default_api::post_v2_parties(&self.config, request)
            .await
            .map_err(|e| format!("allocate party {} failed: {}", hint, describe(e)))?;
        Ok(response.party_details.party)
    }

    /// Creates one contract as `act_as` and returns its contract id.
    pub(crate) async fn create(
        &self,
        act_as: &[&str],
        template_id: &str,
        arguments: Value,
    ) -> Result<String, String> {
        // The OpenAPI generator names the `oneOf` variants by position;
        // `CommandOneOf1` is the create command.
        let command = Command::CommandOneOf1(Box::new(CommandOneOf1::new(CreateCommand::new(
            template_id.to_string(),
            Some(arguments),
        ))));
        let response = self
            .submit(act_as, command, &[])
            .await
            .map_err(|e| format!("create {} failed: {}", template_id, e))?;
        created_cid(&response, template_id)
    }

    /// Exercises one choice as `act_as`, with `disclosed` disclosed.
    pub(crate) async fn exercise(
        &self,
        act_as: &[&str],
        template_id: &str,
        contract_id: &str,
        choice: &str,
        argument: Value,
        disclosed: &[ContractInfo],
    ) -> Result<JsSubmitAndWaitForTransactionResponse, String> {
        // `CommandOneOf3` is the exercise command.
        let command = Command::CommandOneOf3(Box::new(CommandOneOf3::new(ExerciseCommand::new(
            template_id.to_string(),
            contract_id.to_string(),
            choice.to_string(),
            Some(argument),
        ))));
        self.submit(act_as, command, disclosed)
            .await
            .map_err(|e| format!("exercise {} on {} failed: {}", choice, template_id, e))
    }

    /// The party's active contracts of one template.
    pub(crate) async fn active(
        &self,
        party: &str,
        template_id: &str,
    ) -> Result<Vec<JsActiveContract>, String> {
        canton::list_by_template(&self.host, party, &self.token, template_id)
            .await
            .map_err(|e| format!("active {} failed: {}", template_id, e))
    }

    /// Submits one command and waits for its transaction.
    async fn submit(
        &self,
        act_as: &[&str],
        command: Command,
        disclosed: &[ContractInfo],
    ) -> Result<JsSubmitAndWaitForTransactionResponse, String> {
        let commands = JsCommands {
            commands: vec![command],
            command_id: format!("localnet-{}", uuid::Uuid::new_v4()),
            act_as: act_as.iter().map(|party| party.to_string()).collect(),
            user_id: Some(USER_ID.to_string()),
            disclosed_contracts: Some(disclosed.iter().map(disclosed_contract).collect()),
            ..Default::default()
        };
        default_api::post_v2_commands_submit_and_wait_for_transaction(
            &self.config,
            JsSubmitAndWaitForTransactionRequest::new(commands),
        )
        .await
        .map_err(describe)
    }
}

/// A contract the suite discloses, in the shape `ContractInfo` carries.
pub(crate) fn contract_info(contract: &JsActiveContract) -> ContractInfo {
    let created = &contract.created_event;
    ContractInfo {
        contract_id: created.contract_id.clone(),
        template_id: created.template_id.clone(),
        created_event_blob: created.created_event_blob.clone().unwrap_or_default(),
    }
}

/// The id of the `template_id` contract that a transaction created.
pub(crate) fn created_cid(
    response: &JsSubmitAndWaitForTransactionResponse,
    template_id: &str,
) -> Result<String, String> {
    canton::created_by_suffix(response, canton::template_suffix(template_id))
        .map(|created| created.contract_id.clone())
        .ok_or_else(|| format!("No {} was created", template_id))
}

/// A disclosed contract on any synchronizer.
fn disclosed_contract(info: &ContractInfo) -> DisclosedContract {
    DisclosedContract {
        template_id: Some(info.template_id.clone()),
        contract_id: Some(info.contract_id.clone()),
        created_event_blob: info.created_event_blob.clone(),
        synchronizer_id: Some(String::new()),
    }
}

/// The ledger's own error text. The client's `Display` drops the response
/// body, and the body is what says why the ledger refused.
fn describe<T>(error: Error<T>) -> String {
    match error {
        Error::ResponseError(response) => {
            format!("status {}: {}", response.status, response.content)
        }
        other => other.to_string(),
    }
}
