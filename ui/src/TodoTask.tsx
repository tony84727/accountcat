import Checkbox from "@mui/material/Checkbox";
import ListItem from "@mui/material/ListItem";
import ListItemIcon from "@mui/material/ListItemIcon";
import ListItemText from "@mui/material/ListItemText";
import { format } from "date-fns";
import type { Timestamp } from "google-protobuf/google/protobuf/timestamp_pb";
import { type ChangeEvent, useCallback } from "react";
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
	onCompletedChange?(completed: boolean): void;
}
export default function TodoTask({
	task,
	onCompletedChange: setCompleted,
}: Props) {
	const onCheckboxChange = useCallback(
		(event: ChangeEvent<HTMLInputElement>) =>
			setCompleted?.(event.target.checked),
		[setCompleted],
	);
	return (
		<ListItem>
			<ListItemIcon>
				<Checkbox onChange={onCheckboxChange} checked={task.getCompleted()} />
			</ListItemIcon>
			<ListItemText
				sx={{ textDecoration: task.getCompleted() ? "line-through" : "none" }}
				primary={task.getName()}
				secondary={formatTimestamp(task.getCreatedAt())}
			/>
		</ListItem>
	);
}
