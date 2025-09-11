import Paper from "@mui/material/Paper";
import Stack from "@mui/material/Stack";
import type { SxProps } from "@mui/material/styles";
import Typography from "@mui/material/Typography";
import { LineChart } from "@mui/x-charts/LineChart";
import { Empty } from "google-protobuf/google/protobuf/empty_pb";
import { useEffect, useState } from "react";
import { defer, map, Subject, share, takeUntil } from "rxjs";
import { AccountingClient } from "./proto/AccountingServiceClientPb";
import type { DaySpending } from "./proto/accounting_pb";

export default function WeeklyHistogram({ sx }: { sx?: SxProps }) {
	const [dataSet, setDataSet] = useState<DaySpending.AsObject[]>([]);
	const [range, setRange] = useState("");
	useEffect(() => {
		const bye$ = new Subject();
		const accountingClient = new AccountingClient("/api");
		const last7DayHistogram$ = defer(() =>
			accountingClient.getLast7DayHistogram(new Empty()),
		).pipe(share());
		const dataset$ = last7DayHistogram$.pipe(
			map((histogram) => histogram.getDataList().map((x) => x.toObject())),
		);
		const range$ = last7DayHistogram$.pipe(
			map((histogram) => {
				const dataPoints = histogram.getDataList();
				if (!dataPoints.length) {
					return "";
				}
				const first = dataPoints[0];
				const second = dataPoints[dataPoints.length - 1];
				return `(${first.getDate()} ~ ${second.getDate()})`;
			}),
		);
		range$.pipe(takeUntil(bye$)).subscribe(setRange);
		dataset$.pipe(takeUntil(bye$)).subscribe(setDataSet);
		return () => {
			bye$.next(undefined);
		};
	}, []);
	return (
		<Paper elevation={1} sx={{ padding: 1, ...sx }}>
			<Stack>
				<Typography fontWeight="bold">近7日摘要{range}</Typography>
			</Stack>
			<LineChart
				dataset={dataSet}
				xAxis={[
					{ dataKey: "date", scaleType: "band" },
					{ dataKey: "date", scaleType: "band" },
				]}
				series={[
					{
						dataKey: "expense",
						label: "支出",
					},
					{
						dataKey: "income",
						label: "收入",
					},
				]}
			/>
		</Paper>
	);
}
