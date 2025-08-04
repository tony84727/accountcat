import Box from "@mui/material/Box";
import Button from "@mui/material/Button";
import Container from "@mui/material/Container";
import Divider from "@mui/material/Divider";
import Input from "@mui/material/Input";
import List from "@mui/material/List";
import { Empty } from "google-protobuf/google/protobuf/empty_pb";
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
import { NewTask, type Task, TaskUpdate } from "./proto/todolist_pb";
import TodoTask from "./TodoTask";

export default function TodoList() {
	const [tasks, setTasks] = useState<Task[]>();
	const [taskName, setTaskName] = useState<string>("");
	const [onTaskNameInput, setOnTaskNameInput] =
		useState<(event: FormEvent) => void>();
	const [onAddTask, setOnAddTask] = useState<(event: MouseEvent) => void>();
	const [onTaskCompletedChange, setOnTaskCompletedChange] =
		useState<(id: string, completed: boolean) => void>();
	useEffect(() => {
		const todoService = new TodolistClient("/api");
		const bye$ = new Subject();
		const taskNameInput$ = new Subject<FormEvent>();
		const addTask$ = new Subject<MouseEvent>();
		const taskCompletedChange$: Subject<[id: string, completed: boolean]> =
			new Subject();
		setOnTaskNameInput(() => (event: FormEvent) => taskNameInput$.next(event));
		setOnAddTask(() => (event: MouseEvent) => addTask$.next(event));
		setOnTaskCompletedChange(
			() => (id: string, completed?: boolean) =>
				taskCompletedChange$.next([id, completed ?? false]),
		);

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
		const updateTaskResult$ = taskCompletedChange$.pipe(
			switchMap(([id, completed]) => {
				const update = new TaskUpdate();
				update.setId(id);
				update.setCompleted(completed);
				return todoService.updateTask(update);
			}),
		);
		const reloadTasks$ = addTaskResult$.pipe(mergeWith(updateTaskResult$));
		const tasks$ = reloadTasks$.pipe(
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
						<TodoTask
							task={x}
							onCompletedChange={(completed) =>
								onTaskCompletedChange?.(x.getId(), completed)
							}
						/>
						<Divider component="li" />
					</Fragment>
				))}
			</List>
		</Container>
	);
}
