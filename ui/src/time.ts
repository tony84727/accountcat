import { format } from "date-fns";
import type { Timestamp } from "google-protobuf/google/protobuf/timestamp_pb";

export function formatTimestamp(timestamp?: Timestamp): string {
	if (!timestamp) {
		return "";
	}
	const date = timestamp.toDate();
	return format(date, "yyyy-MM-dd hh:mm:ss aa");
}
