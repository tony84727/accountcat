import Paper from "@mui/material/Paper";
import Stack from "@mui/material/Stack";
import type { SxProps } from "@mui/material/styles";
import Typography from "@mui/material/Typography";
import { BarChart } from "@mui/x-charts";
import { Empty } from "google-protobuf/google/protobuf/empty_pb";
import { useEffect, useMemo, useState } from "react";
import { defer, map, Subject, share, takeUntil } from "rxjs";
import { AccountingClient } from "./proto/AccountingServiceClientPb";
import type { MonthlySpending } from "./proto/accounting_pb";

export default function YearlySummary({ sx }: { sx?: SxProps }) {
	const year = useMemo(() => new Date().getFullYear(), []);
	const [dataset, setDataset] = useState<MonthlySpending.AsObject[]>([]);
	useEffect(() => {
		const bye$ = new Subject();
		const accountingClient = new AccountingClient("/api");
		const yearlySummary$ = defer(() =>
			accountingClient.getYearlySummary(new Empty()),
		).pipe(share());
		const dataset$ = yearlySummary$.pipe(
			map((response) => response.getMonthsList().map((x) => x.toObject())),
		);
		dataset$.pipe(takeUntil(bye$)).subscribe(setDataset);
		return () => {
			bye$.next(undefined);
		};
	}, []);
	return (
		<Paper elevation={1} sx={{ padding: 1, ...sx }}>
			<Stack>
				<Typography fontWeight="bold">今年摘要({year})</Typography>
				<BarChart
					dataset={dataset}
					grid={{ horizontal: true }}
					xAxis={[
						{ dataKey: "date", scaleType: "band" },
						{ dataKey: "date", scaleType: "band" },
					]}
					series={[
						{
							dataKey: "expense",
							label: "支出",
							stack: "stack",
						},
						{
							dataKey: "income",
							label: "收入",
							stack: "stack",
						},
					]}
				></BarChart>
			</Stack>
		</Paper>
	);
}
