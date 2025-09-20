import Button from "@mui/material/Button";
import Container from "@mui/material/Container";
import Grid from "@mui/material/Grid";
import Paper from "@mui/material/Paper";
import Snackbar from "@mui/material/Snackbar";
import Stack from "@mui/material/Stack";
import TextField from "@mui/material/TextField";
import Typography from "@mui/material/Typography";
import { Empty } from "google-protobuf/google/protobuf/empty_pb";
import { type FormEventHandler, useEffect, useState } from "react";
import {
	catchError,
	map,
	mergeWith,
	Subject,
	share,
	startWith,
	switchMap,
	take,
	takeUntil,
	timer,
	withLatestFrom,
} from "rxjs";
import { InstanceSettingClient } from "./proto/Instance_settingServiceClientPb";
import { Announcement } from "./proto/instance_setting_pb";
import { createCallback, createNotifier } from "./rxjsutils";

export default function InstanceSetting() {
	const [onContentChange, registerOnContentChange] =
		useState<FormEventHandler<HTMLInputElement | HTMLTextAreaElement>>();
	const [onSave, registerOnSave] = useState<() => void>();
	const [onRevoke, registerOnRevoke] = useState<() => void>();
	const [showSnackbar, setShowSnackbar] = useState(false);
	const [snackbarMessage, setSnackbarMessage] = useState("");
	useEffect(() => {
		const bye$ = new Subject();
		const contentChange$ = createCallback(registerOnContentChange).pipe(
			map((e) => e.currentTarget.value),
		);
		const save$ = createNotifier(registerOnSave);
		const revoke$ = createNotifier(registerOnRevoke);
		const instanceSettingService = new InstanceSettingClient("/api");
		const saveSuccess$ = save$.pipe(
			withLatestFrom(contentChange$),
			switchMap(([, content]) => {
				const announcement = new Announcement();
				announcement.setContent(content);
				return instanceSettingService.setAnnouncement(announcement);
			}),
			share(),
			map(() => true),
			catchError((_err, caught) => caught.pipe(startWith(false))),
		);
		const revokeSuccess$ = revoke$.pipe(
			switchMap(() => instanceSettingService.revokeAnnouncement(new Empty())),
			share(),
			map(() => true),
			catchError((_err, caught) => caught.pipe(startWith(false))),
		);
		const showSnackbar$ = saveSuccess$.pipe(
			mergeWith(revokeSuccess$),
			switchMap(() =>
				timer(5000).pipe(
					map(() => false),
					take(1),
					startWith(true),
				),
			),
		);
		const snackbarMessage$ = saveSuccess$.pipe(
			map((result) => (result ? "公告設定成功" : "公告設定失敗")),
			mergeWith(
				revokeSuccess$.pipe(
					map((result) => (result ? "公告撤銷成功" : "公告撤銷失敗")),
				),
			),
		);
		showSnackbar$.pipe(takeUntil(bye$)).subscribe(setShowSnackbar);
		snackbarMessage$.pipe(takeUntil(bye$)).subscribe(setSnackbarMessage);
		return () => bye$.next(undefined);
	}, []);
	return (
		<Container>
			<Snackbar message={snackbarMessage} open={showSnackbar} />
			<Paper elevation={1}>
				<Stack padding={1}>
					<Typography fontWeight="bold" padding={1}>
						公告
					</Typography>
					<TextField
						label="訊息"
						variant="outlined"
						onChange={onContentChange}
					/>
					<Grid container justifyContent="center" padding={1} gap={1}>
						<Button color="primary" variant="contained" onClick={onSave}>
							儲存
						</Button>
						<Button color="error" variant="contained" onClick={onRevoke}>
							撤銷目前公告
						</Button>
					</Grid>
				</Stack>
			</Paper>
		</Container>
	);
}
