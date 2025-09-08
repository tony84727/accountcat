import Container from "@mui/material/Container";
import Divider from "@mui/material/Divider";
import Grid from "@mui/material/Grid";
import Paper from "@mui/material/Paper";
import Stack from "@mui/material/Stack";
import type { Palette, PaletteColor } from "@mui/material/styles";
import Typography from "@mui/material/Typography";
import { Empty } from "google-protobuf/google/protobuf/empty_pb";
import { useEffect, useState } from "react";
import { defer, map, Subject, share, takeUntil } from "rxjs";
import { AccountingClient } from "./proto/AccountingServiceClientPb";

type PaletteColorKeys = {
	[K in keyof Palette]-?: Palette[K] extends PaletteColor ? K : never;
}[keyof Palette];

interface ColorBlockProps {
	label: string;
	children: string;
	color: PaletteColorKeys;
}

function ColorBlock({ label, children, color }: ColorBlockProps) {
	return (
		<Stack
			sx={{
				backgroundColor: (theme) => `${theme.palette[color].light}55`,
				borderRadius: 1,
				padding: 1,
				flexBasis: 1,
				flexGrow: 1,
			}}
		>
			<Typography fontWeight="bold">{label}</Typography>
			<Typography>
				<Typography component="span" fontWeight="bold" variant="subtitle2">
					$
				</Typography>
				{children}
			</Typography>
		</Stack>
	);
}

export default function Insight() {
	const [income, setIncome] = useState("0");
	const [expense, setExpense] = useState("0");
	const [date, setDate] = useState("");
	const [count, setCount] = useState(0);
	useEffect(() => {
		const bye$ = new Subject();
		const accountingService = new AccountingClient("/api");
		const dailySpending$ = defer(() =>
			accountingService.getDailySpending(new Empty()),
		).pipe(share());
		dailySpending$
			.pipe(
				takeUntil(bye$),
				map((response) => response.getIncome()),
			)
			.subscribe(setIncome);
		dailySpending$
			.pipe(
				takeUntil(bye$),
				map((response) => response.getExpense()),
			)
			.subscribe(setExpense);
		dailySpending$
			.pipe(
				takeUntil(bye$),
				map((response) => response.getCount()),
			)
			.subscribe(setCount);
		dailySpending$
			.pipe(
				takeUntil(bye$),
				map((response) => response.getDate()),
			)
			.subscribe(setDate);
		return () => {
			bye$.next(undefined);
		};
	}, []);
	return (
		<Container>
			<Typography variant="h4">近期消費</Typography>
			<Divider sx={{ marginY: 1 }}></Divider>
			<Paper elevation={1} sx={{ padding: 1 }}>
				<Stack>
					<Typography fontWeight="bold">
						本日摘要{date && `(${date})`}
					</Typography>
					<Grid container gap={1}>
						<ColorBlock label="收入" color="success">
							{income}
						</ColorBlock>
						<ColorBlock label="支出" color="error">
							{expense}
						</ColorBlock>
					</Grid>
					<Typography variant="subtitle1">共{count}筆交易</Typography>
				</Stack>
			</Paper>
		</Container>
	);
}
