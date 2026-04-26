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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_proto_timestamp() {
        let datetime = OffsetDateTime::from_unix_timestamp(1600000000).unwrap();
        let timestamp = to_proto_timestamp(datetime);
        assert_eq!(timestamp.seconds, 1600000000);
        assert_eq!(timestamp.nanos, 0);
    }

    #[test]
    fn test_to_proto_timestamp_with_nanos() {
        let datetime = OffsetDateTime::from_unix_timestamp_nanos(1600000000123456789).unwrap();
        let timestamp = to_proto_timestamp(datetime);
        assert_eq!(timestamp.seconds, 1600000000);
        assert_eq!(timestamp.nanos, 123456789);
    }

    #[test]
    fn test_from_proto_timestamp_negative() {
        let timestamp = Timestamp {
            seconds: -1600000000,
            nanos: 0,
        };
        let datetime = from_proto_timestamp(timestamp).unwrap();
        assert_eq!(datetime.unix_timestamp(), -1600000000);
    }

    #[test]
    fn test_from_proto_timestamp() {
        let timestamp = Timestamp {
            seconds: 1600000000,
            nanos: 0,
        };
        let datetime = from_proto_timestamp(timestamp).unwrap();
        assert_eq!(datetime.unix_timestamp(), 1600000000);
    }

    #[test]
    fn test_from_proto_timestamp_out_of_bounds() {
        let timestamp = Timestamp {
            seconds: i64::MAX,
            nanos: 0,
        };
        let result = from_proto_timestamp(timestamp);
        assert!(result.is_err());
    }
}
