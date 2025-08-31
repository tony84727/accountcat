use prost_types::Timestamp;
use time::OffsetDateTime;

pub fn to_proto_timestamp(datetime: OffsetDateTime) -> Timestamp {
    Timestamp {
        seconds: datetime.unix_timestamp(),
        nanos: datetime.nanosecond() as i32,
    }
}

pub fn from_proto_timestamp(
    timestamp: Timestamp,
) -> Result<OffsetDateTime, time::error::ComponentRange> {
    OffsetDateTime::from_unix_timestamp(timestamp.seconds)
}
