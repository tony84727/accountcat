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
import { format } from "date-fns";
import { Empty } from "google-protobuf/google/protobuf/empty_pb";
import type { Timestamp } from "google-protobuf/google/protobuf/timestamp_pb";
import type { MouseEvent } from "react";
import { type FormEvent, Fragment, useEffect, useState } from "react";
import {
	defer,
	filter,
	map,
	mergeWith,
	type Observable,
	Subject,
	share,
	startWith,
	switchMap,
	takeUntil,
	withLatestFrom,
} from "rxjs";
import { TodolistClient } from "./proto/TodolistServiceClientPb";
import { NewTask, type Task } from "./proto/todolist_pb";

function formatTimestamp(timestamp?: Timestamp): string {
	if (!timestamp) {
		return "";
	}
	const date = timestamp.toDate();
	return format(date, "yyyy-MM-dd hh:mm:ss aa");
}
export default function TodoList() {
	const [tasks, setTasks] = useState<Task[]>();
	const [taskName, setTaskName] = useState<string>("");
	const [onTaskNameInput, setOnTaskNameInput] =
		useState<(event: FormEvent) => void>();
	const [onAddTask, setOnAddTask] = useState<(event: MouseEvent) => void>();
	useEffect(() => {
		const todoService = new TodolistClient("/api");
		const bye$ = new Subject();
		const taskNameInput$ = new Subject<FormEvent>();
		setOnTaskNameInput(() => (event: FormEvent) => taskNameInput$.next(event));
		const addTask$ = new Subject<MouseEvent>();
		setOnAddTask(() => (event: MouseEvent) => addTask$.next(event));

		const taskName$: Observable<string> = taskNameInput$.pipe(
			map((event) => (event.target as HTMLInputElement).value),
			mergeWith(defer(() => addTaskResult$).pipe(map(() => ""))),
		);
		const addTaskResult$ = addTask$.pipe(
			withLatestFrom(taskName$),
			map(([, name]) => name),
			filter(Boolean),
			switchMap((name) => {
				const request = new NewTask();
				request.setName(name);
				return todoService.add(request);
			}),
			share(),
		);
		const tasks$ = addTaskResult$.pipe(
			startWith(undefined),
			switchMap(() => todoService.list(new Empty())),
			share(),
			map((response) => response.getTasksList()),
		);

		taskName$.pipe(takeUntil(bye$)).subscribe(setTaskName);
		tasks$.pipe(takeUntil(bye$)).subscribe(setTasks);
		return () => {
			bye$.next(undefined);
			bye$.complete();
		};
	}, []);
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
				{(tasks ?? []).map((x, i) => (
					<Fragment key={`${i}${x}`}>
						<ListItem>
							<ListItemIcon>
								<Checkbox />
							</ListItemIcon>
							<ListItemText
								primary={x.getName()}
								secondary={formatTimestamp(x.getCreatedAt())}
							/>
						</ListItem>
						<Divider component="li" />
					</Fragment>
				))}
			</List>
		</Container>
	);
}
