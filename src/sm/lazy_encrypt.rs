use alloc::boxed::Box;
use super::Error;
use core::fmt::{Display,Debug};
use core::future::Future;
use core::marker::Unpin;
use core::pin::Pin;
use core::task::{Context,Poll};
use core::time::Duration;
use crate::hci::{
    cb::set_event_mask::EventMask,
    common::ConnectionHandle,
    events::{Events, EventsData, LEMeta},
    HostInterface,
    HostControllerInterface,
    le::encryption::start_encryption::Parameter as EncryptionParameter,
};

type MasterEventMask = [EventMask; 3];
type SlaveEventMask = ([EventMask; 1], [LEMeta; 1]);

pub fn new_lazy_encrypt_master_future<'a, HCI, D>(
    event_mask: MasterEventMask,
    ltk: u128,
    hci: &'a HostInterface<HCI>,
    connection_handle: ConnectionHandle,
    encryption_timeout: D,
) -> impl Future<Output = Result<(), Error>> + 'a
    where HCI: HostControllerInterface + 'static,
          D: Into<Option<Duration>>,
          <HCI as HostControllerInterface>::ReceiveEventError: core::marker::Unpin
{
    MasterLazyEncryptFuture::new(
        event_mask,
        ltk,
        hci,
        connection_handle,
        crate::hci::cb::set_event_mask::send,
        crate::hci::le::encryption::start_encryption::send,
        crate::hci::HostInterface::wait_for_event_with_matcher,
        encryption_timeout,
    )
}

struct MasterLazyEncryptFuture<'a, HCI, F1, SMFn, F2, SEFn, WFEFn, F3> {
    event_mask: MasterEventMask,
    ltk: u128,
    hci: &'a HostInterface<HCI>,
    connection_handle: ConnectionHandle,
    current: MasterLazyEncryptCurrent<F1,F2,F3,F3>,
    set_mask_fn: SMFn,
    start_encryption_fn: SEFn,
    wait_for_event_with_matcher_fn: WFEFn,
    encrypt_timeout: Option<Duration>,
}

impl<'a, HCI, F1, SMFn, F2, SEFn, WFEFn, F3>
MasterLazyEncryptFuture<'a, HCI, F1, SMFn, F2, SEFn, WFEFn, F3>
    where HCI: HostControllerInterface
{
    fn new<D: Into<Option<Duration>>>(
        event_mask: MasterEventMask,
        ltk: u128,
        hci: &'a HostInterface<HCI>,
        connection_handle: ConnectionHandle,
        set_mask_fn: SMFn,
        start_encryption_fn: SEFn,
        wait_for_event_with_matcher_fn: WFEFn,
        encrypt_timeout: D,
    ) -> Self {

        MasterLazyEncryptFuture {
            event_mask,
            ltk,
            hci,
            connection_handle,
            current: MasterLazyEncryptCurrent::None,
            set_mask_fn,
            start_encryption_fn,
            wait_for_event_with_matcher_fn,
            encrypt_timeout: encrypt_timeout.into()
        }
    }
}

impl<'a, HCI, F1, FER1, SMFn, F2, FER2, SEFn, WFEFn, F3> core::future::Future
for MasterLazyEncryptFuture<'a, HCI, F1, SMFn, F2, SEFn, WFEFn, F3,>
    where  HCI: HostControllerInterface,
           F1: Future<Output = Result<(), FER1>> + Unpin + 'a,
           FER1: Display + Debug + 'static,
           SMFn: Fn(&'a HostInterface<HCI>, &[EventMask]) -> F1 + Unpin,
           F2: Future<Output = Result<(), FER2>> + Unpin + 'a,
           FER2: Display + Debug + 'static,
           SEFn: Fn(&'a HostInterface<HCI>, EncryptionParameter) -> F2 + Unpin,
           F3: Future<Output = Result<EventsData, <HCI as HostControllerInterface>::ReceiveEventError>> + Unpin + 'a,
           WFEFn: Fn(&'a HostInterface<HCI>, Events, Option<Duration>, MasterEncryptEventMatcher) -> F3 + Unpin,
           <HCI as HostControllerInterface>::ReceiveEventError: 'static + Unpin
{
    type Output = Result<(), Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output>
    {
        let this = self.get_mut();

        loop {
            match &mut this.current {
                MasterLazyEncryptCurrent::None => {
                    let future = (this.set_mask_fn)(this.hci, &this.event_mask);

                    this.current = MasterLazyEncryptCurrent::SetMask(future);
                },
                MasterLazyEncryptCurrent::SetMask(future) => {

                    match Pin::new(future).poll(cx).map_err(err_map) {
                        Poll::Pending => break Poll::Pending,
                        err @ Poll::Ready(Err(_)) => break err,
                        Poll::Ready(Ok(_)) => {

                            let encrypt_pram = EncryptionParameter {
                                handle: this.connection_handle,
                                random_number: 0,
                                encrypted_diversifier: 0,
                                long_term_key: this.ltk
                            };

                            let start_encrypt_fut = (this.start_encryption_fn)(this.hci, encrypt_pram);

                            let encrypt_change_fut = (this.wait_for_event_with_matcher_fn)(
                                this.hci,
                                Events::EncryptionChange,
                                this.encrypt_timeout,
                                MasterEncryptEventMatcher(this.connection_handle),
                            );

                            let encrypt_key_refresh_fut = (this.wait_for_event_with_matcher_fn)(
                                this.hci,
                                Events::EncryptionKeyRefreshComplete,
                                this.encrypt_timeout,
                                MasterEncryptEventMatcher(this.connection_handle),
                            );

                            this.current = MasterLazyEncryptCurrent::StartEncryption(
                                start_encrypt_fut,
                                encrypt_change_fut,
                                encrypt_key_refresh_fut,
                            );
                        },
                    }
                },
                MasterLazyEncryptCurrent::StartEncryption(cmd_fut, _, _) => {

                    match Pin::new(cmd_fut).poll(cx).map_err(err_map) {
                        Poll::Pending => break Poll::Pending,
                        err @ Poll::Ready(Err(_)) => break err,
                        Poll::Ready(Ok(_)) => {

                            let start_encrypt = core::mem::replace(
                                &mut this.current,
                                MasterLazyEncryptCurrent::None
                            );

                            match start_encrypt {
                                MasterLazyEncryptCurrent::StartEncryption(_, f1, f2) =>
                                    this.current = MasterLazyEncryptCurrent::AwaitEncryptFinish(f1, f2),
                                _ => panic!("Expected StartEncryption")
                            }

                        },
                    }
                },
                MasterLazyEncryptCurrent::AwaitEncryptFinish(change_fut, refresh_fut) => {

                    match Pin::new(change_fut).poll(cx).map_err(err_map) {
                        Poll::Pending => match Pin::new(refresh_fut).poll(cx).map_err(err_map) {
                            Poll::Pending => break Poll::Pending,
                            Poll::Ready(Err(e)) => break Poll::Ready(Err(e)),
                            Poll::Ready(Ok(_)) => break Poll::Ready(Ok(())),
                        },
                        Poll::Ready(Err(e)) => break Poll::Ready(Err(e)),
                        Poll::Ready(Ok(change_data)) => {
                            match change_data {
                                EventsData::EncryptionChange(e_data) => {

                                    break match e_data.encryption_enabled.get_for_le() {
                                        crate::hci::common::EncryptionLevel::AESCCM =>
                                            Poll::Ready(Ok(())),
                                        crate::hci::common::EncryptionLevel::E0 =>
                                            Poll::Ready(Err(err_map("E0 cypher used"))),
                                        crate::hci::common::EncryptionLevel::Off =>
                                            Poll::Ready(Err(err_map("Encryption not enabled"))),
                                    };
                                }
                                ed => panic!("Received unexpected event data: '{:?}'", ed),
                            }
                        },
                    }
                }
            }
        }
    }
}

enum MasterLazyEncryptCurrent<F1,F2,F3,F4>
{
    None,
    SetMask(F1),
    StartEncryption(F2,F3,F4),
    AwaitEncryptFinish(F3, F4)
}

struct MasterEncryptEventMatcher(ConnectionHandle);

impl crate::hci::EventMatcher for MasterEncryptEventMatcher {

    fn match_event(&self, event_data: &EventsData) -> bool {
        match event_data {
            EventsData::EncryptionKeyRefreshComplete(data) => data.connection_handle == self.0,
            EventsData::EncryptionChange(data) => data.connection_handle == self.0,
            _ => false,
        }
    }
}

fn err_map<E: 'static>(e: E) -> super::Error where E: Debug {
    super::Error::EncryptionFailed(Box::new(e))
}

pub fn new_await_encrypt_slave_future<'a, HCI, D, LTK>(
    ltk: LTK,
    event_mask: SlaveEventMask,
    hci: &'a HostInterface<HCI>,
    connection_handle: ConnectionHandle,
    timeout: D,
) -> impl Future<Output = Result<(), Error>> + 'a
    where HCI: HostControllerInterface + 'static,
            D: Into<Option<Duration>>,
          LTK: Into<Option<u128>>,
          <HCI as HostControllerInterface>::ReceiveEventError: 'static + Unpin
{
    SlaveLazyEncryptFuture {
        hci,
        event_mask,
        ltk_neg_fn: crate::hci::le::encryption::long_term_key_request_negative_reply::send,
        ltk_pos_fn: crate::hci::le::encryption::long_term_key_request_reply::send,
        set_event_fn: crate::hci::cb::set_event_mask::send,
        set_le_event_fn: crate::hci::le::mandatory::set_event_mask::send,
        wait_for_event_with_matcher_fn: crate::hci::HostInterface::wait_for_event_with_matcher,
        connection_handle,
        ltk_event_timeout: timeout.into(),
        current: SlaveLazyEncryptCurrent::None,
        ltk: ltk.into(),
    }
}

struct SlaveLazyEncryptFuture<'a, HCI, FNeg, FPos, FutEm, FutLEm, FutLTKE, FutNeg, FutPos, FE, FLE,
    WFEFn>
{
    //sm_server: &'a super::SecurityManager,
    hci: &'a HostInterface<HCI>,
    event_mask: SlaveEventMask,
    ltk_neg_fn: FNeg,
    ltk_pos_fn: FPos,
    set_event_fn: FE,
    set_le_event_fn: FLE,
    wait_for_event_with_matcher_fn: WFEFn,
    ltk_event_timeout: Option<Duration>,
    connection_handle: ConnectionHandle,
    current: SlaveLazyEncryptCurrent<FutEm, FutLEm, FutLTKE, FutNeg, FutPos>,
    ltk: Option<u128>,
}

impl <'a, HCI, FNeg, FNegRet, FNegE, FPos, FPosRet, FPosE, FutEm, FutLEm, FutLTKE, FutNeg, FutPos,
    FE, FLE, FutEmE, FutLEmE, WFEFn>
Future
for SlaveLazyEncryptFuture<'a, HCI, FNeg, FPos, FutEm, FutLEm, FutLTKE, FutNeg, FutPos, FE, FLE, WFEFn>
    where  HCI: HostControllerInterface,
         FutEm: Future<Output=Result<(), FutEmE>> + Unpin + 'a,
        FutLEm: Future<Output=Result<(), FutLEmE>> + Unpin + 'a,
       FutLTKE: Future<Output = Result<EventsData, <HCI as HostControllerInterface>::ReceiveEventError>> + Unpin + 'a,
        FutNeg: Future<Output=Result<FNegRet, FNegE>> + Unpin + 'a,
        FutPos: Future<Output=Result<FPosRet, FPosE>> + Unpin + 'a,
            FE: Fn(&'a HostInterface<HCI>, &[EventMask]) -> FutEm + Unpin,
           FLE: Fn(&'a HostInterface<HCI>, &[LEMeta]) -> FutLEm + Unpin,
         WFEFn: Fn(&'a HostInterface<HCI>, Events, Option<Duration>, SlaveLTKEventMatcher) -> FutLTKE + Unpin,
          FNeg: Fn(&'a HostInterface<HCI>, ConnectionHandle) -> FutNeg + Unpin,
          FPos: Fn(&'a HostInterface<HCI>, ConnectionHandle, u128) -> FutPos + Unpin,
        FutEmE: Debug + Display + 'static,
       FutLEmE: Debug + Display + 'static,
         FNegE: Debug + Display + 'static,
         FPosE: Debug + Display + 'static,
          <HCI as HostControllerInterface>::ReceiveEventError: 'static + Unpin
{
    type Output = Result<(), Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.get_mut();

        loop {
            match &mut this.current {
                SlaveLazyEncryptCurrent::None => {
                    let future = (this.set_event_fn)(this.hci, &this.event_mask.0);

                    this.current = SlaveLazyEncryptCurrent::SetEventMask(future);
                }
                SlaveLazyEncryptCurrent::SetEventMask(future) => {
                    match Pin::new(future).poll(cx).map_err(err_map) {
                        Poll::Pending => break Poll::Pending,
                        err @ Poll::Ready(Err(_)) => break err,
                        Poll::Ready(Ok(_)) => {
                            let future = (this.set_le_event_fn)(this.hci, &this.event_mask.1);

                            this.current = SlaveLazyEncryptCurrent::SetLeEventMask(future);
                        }
                    }
                }
                SlaveLazyEncryptCurrent::SetLeEventMask(future)=> {
                    match Pin::new(future).poll(cx).map_err(err_map) {
                        Poll::Pending => break Poll::Pending,
                        err @ Poll::Ready(Err(_)) => break err,
                        Poll::Ready(Ok(_)) => {
                            let ltk_event_fut = (this.wait_for_event_with_matcher_fn)(
                                this.hci,
                                LEMeta::LongTermKeyRequest.into(),
                                this.ltk_event_timeout,
                                SlaveLTKEventMatcher(this.connection_handle),
                            );

                            this.current = SlaveLazyEncryptCurrent::AwaitLTKReq(ltk_event_fut);
                        }
                    }
                }
                SlaveLazyEncryptCurrent::AwaitLTKReq(future) => {
                    match Pin::new(future).poll(cx).map_err(err_map) {
                        Poll::Pending => break Poll::Pending,
                        Poll::Ready(Err(e)) => break Poll::Ready(Err(e)),
                        Poll::Ready(Ok(_)) => {
                            match this.ltk {
                                Some(ltk) => {
                                    let future = (this.ltk_pos_fn)(this.hci, this.connection_handle, ltk);

                                    this.current = SlaveLazyEncryptCurrent::Positive(future);
                                }
                                None => {
                                    let future = (this.ltk_neg_fn)(this.hci, this.connection_handle);

                                    this.current = SlaveLazyEncryptCurrent::Negative(future);
                                }
                            }
                        }
                    }
                }
                SlaveLazyEncryptCurrent::Positive(future) => {
                    match Pin::new(future).poll(cx).map_err(err_map) {
                        Poll::Pending => break Poll::Pending,
                        Poll::Ready(Err(e)) => break Poll::Ready(Err(e)),
                        Poll::Ready(Ok(_)) => break Poll::Ready(Ok(())),
                    }
                }
                SlaveLazyEncryptCurrent::Negative(future) => {
                    match Pin::new(future).poll(cx).map_err(err_map) {
                        Poll::Pending => break Poll::Pending,
                        Poll::Ready(Err(e)) => break Poll::Ready(Err(e)),
                        Poll::Ready(Ok(_)) => break Poll::Ready(Ok(())),
                    }
                }
            }
        }
    }
}

enum SlaveLazyEncryptCurrent<FutEm, FutLEm, FutLTKE, FutNeg, FutPos> {
    None,
    SetEventMask(FutEm),
    SetLeEventMask(FutLEm),
    AwaitLTKReq(FutLTKE),
    Negative(FutNeg),
    Positive(FutPos),
}

struct SlaveLTKEventMatcher(ConnectionHandle);

impl crate::hci::EventMatcher for SlaveLTKEventMatcher {

    fn match_event(&self, event_data: &EventsData) -> bool {
        use crate::hci::events::LEMetaData;

        match event_data {
            EventsData::LEMeta(LEMetaData::LongTermKeyRequest(data)) =>
                data.connection_handle == self.0,
            _ => false,
        }
    }
}