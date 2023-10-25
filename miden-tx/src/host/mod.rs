use vm_processor::{
    AdviceExtractor, AdviceInjector, AdviceProvider, ExecutionError, Host, HostResponse,
    ProcessState,
};

mod event;
use event::EventHandler;

pub struct TransactionHost<A> {
    adv_provider: A,
    event_handler: EventHandler,
}

impl<A: AdviceProvider> TransactionHost<A> {
    pub fn new(adv_provider: A) -> Self {
        Self {
            adv_provider,
            event_handler: EventHandler::default(),
        }
    }

    pub fn into_parts(self) -> (A, EventHandler) {
        (self.adv_provider, self.event_handler)
    }
}

impl<A: AdviceProvider> Host for TransactionHost<A> {
    fn get_advice<S: ProcessState>(
        &mut self,
        process: &S,
        extractor: AdviceExtractor,
    ) -> Result<HostResponse, ExecutionError> {
        self.adv_provider.get_advice(process, &extractor)
    }

    fn set_advice<S: ProcessState>(
        &mut self,
        process: &S,
        injector: AdviceInjector,
    ) -> Result<HostResponse, ExecutionError> {
        self.adv_provider.set_advice(process, &injector)
    }

    fn on_event<S: ProcessState>(
        &mut self,
        process: &S,
        event_id: u32,
    ) -> Result<HostResponse, ExecutionError> {
        self.event_handler.handle_event(process, event_id)
    }
}
