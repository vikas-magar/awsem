use tokio::sync::broadcast;

#[derive(Clone, Debug)]
pub enum BusEvent {
    S3Notification {
        bucket: String,
        key: String,
        target_arn: String,
        target_type: String,
    },
    LambdaInvocation {
        function_arn: String,
        payload: String,
    },
}

pub fn new_bus(capacity: usize) -> (broadcast::Sender<BusEvent>, broadcast::Receiver<BusEvent>) {
    broadcast::channel(capacity)
}
