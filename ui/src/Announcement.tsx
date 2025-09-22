import Box from "@mui/material/Box";
import Typography from "@mui/material/Typography";
import { Empty } from "google-protobuf/google/protobuf/empty_pb";
import { useEffect, useState } from "react";
import {
	EMPTY,
	map,
	type Observable,
	Subject,
	share,
	startWith,
	switchMap,
	takeUntil,
} from "rxjs";
import { UserClient } from "./proto/UserServiceClientPb";

interface Props {
	reload$?: Observable<unknown>;
}

export default function Announcement({ reload$ }: Props) {
	const [content, setContent] = useState("");
	useEffect(() => {
		const bye$ = new Subject();
		const userService = new UserClient("/api");
		const param$ = (reload$ ?? EMPTY).pipe(
			startWith(undefined),
			switchMap(() => userService.getParam(new Empty())),
			share(),
			map((x) => x.getAnnouncement()),
		);
		param$.pipe(takeUntil(bye$)).subscribe(setContent);
		return () => bye$.next(undefined);
	}, [reload$]);
	return (
		content && (
			<Box
				sx={{
					backgroundColor: (theme) => theme.palette.warning.light,
					marginBottom: 1,
					justifyContent: "center",
					display: "flex",
				}}
			>
				<Typography variant="h5">{content}</Typography>
			</Box>
		)
	);
}
