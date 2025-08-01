import Box from "@mui/material/Box";
import Button from "@mui/material/Button";
import Checkbox from "@mui/material/Checkbox";
import Container from "@mui/material/Container";
import Divider from "@mui/material/Divider";
import Input from "@mui/material/Input";
import List from "@mui/material/List";
import ListItem from "@mui/material/ListItem";
import ListItemIcon from "@mui/material/ListItemIcon";
import ListItemText from "@mui/material/ListItemText";
import { type FormEvent, useCallback, useState } from "react";
export default function TodoList() {
	const [tasks, setTasks] = useState<string[]>([]);
	const [taskName, setTaskName] = useState<string>("");
	const onTaskNameInput = useCallback((event: FormEvent<HTMLInputElement>) => {
		setTaskName((event.target as HTMLInputElement).value);
	}, []);
	const onAddTask = useCallback(() => {
		if (!taskName) {
			return;
		}
		setTasks((x) => [...x, taskName]);
		setTaskName("");
	}, [taskName]);
	return (
		<Container>
			<Box>
				<Input
					type="text"
					placeholder="任務名稱"
					value={taskName}
					onInput={onTaskNameInput}
				/>
				<Button onClick={onAddTask}>新增任務</Button>
			</Box>

			<List>
				{tasks.map((x, i) => (
					<>
						<ListItem key={`${i}${x}`}>
							<ListItemIcon>
								<Checkbox />
							</ListItemIcon>
							<ListItemText primary={x} secondary="date" />
						</ListItem>
						<Divider component="li" />
					</>
				))}
			</List>
		</Container>
	);
}
