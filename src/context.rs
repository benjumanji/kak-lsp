use crate::types::*;
use crossbeam_channel::Sender;
use fnv::FnvHashMap;
use jsonrpc_core::{self, Call, Id, Params, Version};
use lsp_types::*;
use std::fs;

pub struct Context {
    pub capabilities: Option<ServerCapabilities>,
    pub config: Config,
    pub diagnostics: FnvHashMap<String, Vec<Diagnostic>>,
    pub editor_tx: Sender<EditorResponse>,
    pub lang_srv_tx: Sender<ServerMessage>,
    pub language_id: String,
    pub pending_requests: Vec<EditorRequest>,
    pub request_counter: u64,
    pub response_waitlist: FnvHashMap<Id, (EditorMeta, String, EditorParams)>,
    pub root_path: String,
    pub session: SessionId,
    pub versions: FnvHashMap<String, u64>,
}

impl Context {
    pub fn new(
        language_id: &str,
        initial_request: EditorRequest,
        lang_srv_tx: Sender<ServerMessage>,
        editor_tx: Sender<EditorResponse>,
        config: Config,
        root_path: String,
    ) -> Self {
        let session = initial_request.meta.session.clone();
        Context {
            capabilities: None,
            config,
            diagnostics: FnvHashMap::default(),
            editor_tx,
            lang_srv_tx,
            language_id: language_id.to_string(),
            pending_requests: vec![initial_request],
            request_counter: 0,
            response_waitlist: FnvHashMap::default(),
            root_path,
            session,
            versions: FnvHashMap::default(),
        }
    }

    pub fn call(&mut self, id: Id, method: String, params: impl ToParams) {
        let params = params.to_params();
        if params.is_err() {
            error!("Failed to convert params");
            return;
        }
        let call = jsonrpc_core::MethodCall {
            jsonrpc: Some(Version::V2),
            id,
            method,
            params: Some(params.unwrap()),
        };
        self.lang_srv_tx
            .send(ServerMessage::Request(Call::MethodCall(call)));
    }

    pub fn notify(&mut self, method: String, params: impl ToParams) {
        let params = params.to_params();
        if params.is_err() {
            error!("Failed to convert params");
            return;
        }
        let notification = jsonrpc_core::Notification {
            jsonrpc: Some(Version::V2),
            method,
            // NOTE this is required because jsonrpc serializer converts Some(None) into []
            params: match params.unwrap() {
                Params::None => None,
                params => Some(params),
            },
        };
        self.lang_srv_tx
            .send(ServerMessage::Request(Call::Notification(notification)))
    }

    pub fn exec(&self, meta: EditorMeta, command: String) {
        match meta.fifo {
            Some(fifo) => fs::write(fifo, command).expect("Failed to write command to fifo"),
            None => self.editor_tx.send(EditorResponse { meta, command }),
        }
    }

    pub fn next_request_id(&mut self) -> Id {
        let id = Id::Num(self.request_counter);
        self.request_counter += 1;
        id
    }
}
