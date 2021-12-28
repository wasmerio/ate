use wasm_bus::macros::*;

#[wasm_bus(format = "json")]
pub trait Time {
    fn sleep(duration_ms: u128);
}

/*
#[derive(Debug, Clone, serde :: Serialize, serde :: Deserialize)]
pub struct TimeSleepRequest
{
    pub duration_ms : u128
}

pub mod detail
{
    #[must_use = "the method is not invoked until you call the 'call' method"]
    pub struct TimeSleepBuilder {
        pub(super) builder : wasm_bus :: abi :: CallBuilder
    }

    impl TimeSleepBuilder
    {
        #[doc = r" Upon receiving a particular message from the service that is"]
        #[doc = r" invoked this callback will take some action"] #[doc = r""]
        #[doc = r" Note: This must be called before the invoke or things will go wrong"]
        #[doc = r" hence there is a builder that invokes this in the right order"]
        pub fn callback < C, F > (& mut self, callback : F) -> & mut Self
        where C : serde :: Serialize + serde :: de :: DeserializeOwned + Send + Sync + 'static, F : FnMut(C), F : Send + 'static,
        {
            self.builder.callback(callback);
            self
        }

        #[doc = r" Invokes the call and allows the caller to wait for the result"]
        pub fn call(self) -> TimeSleepCall
        {
            TimeSleepCall {
                task : self.builder.invoke()
            }
        }

        #[doc = r" Allow the caller to wait for the result of the invocation"]
        pub fn join(self) -> wasm_bus :: abi :: CallJoin < () >
        {
            self.call().join()
        }
    }

    pub struct TimeSleepCall
    {
        task : wasm_bus :: abi :: Call
    }

    impl TimeSleepCall
    {
        #[doc = r" Allow the caller to wait for the result of the invocation"]
        pub fn join(self) -> wasm_bus :: abi :: CallJoin < () >
        {
            self.task.join()
        }
    }
}

pub struct Time {
    #[allow(unused)]
    task : wasm_bus :: abi :: Call
}

impl Time
{
    pub fn sleep(wapm : & str, duration_ms : u128) -> detail::TimeSleepBuilder
    {
        let request = TimeSleepRequest { duration_ms } ;
        detail::TimeSleepBuilder
        {
            builder : wasm_bus :: abi :: call(wapm.to_string().into(), request)
        }
    }
}
*/
