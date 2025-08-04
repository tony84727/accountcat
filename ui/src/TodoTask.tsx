import Checkbox from "@mui/material/Checkbox";
import ListItem from "@mui/material/ListItem";
import ListItemIcon from "@mui/material/ListItemIcon";
import ListItemText from "@mui/material/ListItemText";
import { format } from "date-fns";
import type { Timestamp } from "google-protobuf/google/protobuf/timestamp_pb";
import type { Task } from "./proto/todolist_pb";

function formatTimestamp(timestamp?: Timestamp): string {
	if (!timestamp) {
		return "";
	}
	const date = timestamp.toDate();
	return format(date, "yyyy-MM-dd hh:mm:ss aa");
}

interface Props {
	task: Task;
}
export default function TodoTask({ task }: Props) {
	return (
		<ListItem>
			<ListItemIcon>
				<Checkbox />
			</ListItemIcon>
			<ListItemText
				primary={task.getName()}
				secondary={formatTimestamp(task.getCreatedAt())}
			/>
		</ListItem>
	);
}
