use prost_types::Timestamp;
use sqlx::types::time::OffsetDateTime;

pub fn to_proto_timestamp(datetime: OffsetDateTime) -> Timestamp {
    Timestamp {
        seconds: datetime.unix_timestamp(),
        nanos: datetime.nanosecond() as i32,
    }
}
