import AddCircleOutlineIcon from "@mui/icons-material/AddCircleOutline";
import Button from "@mui/material/Button";
import Container from "@mui/material/Container";
import Grid from "@mui/material/Grid";
import Table from "@mui/material/Table";
import TableBody from "@mui/material/TableBody";
import TableCell from "@mui/material/TableCell";
import TableHead from "@mui/material/TableHead";
import TableRow from "@mui/material/TableRow";
import TextField from "@mui/material/TextField";
import { Empty } from "google-protobuf/google/protobuf/empty_pb";
import {
	type FormEvent,
	type FormEventHandler,
	useEffect,
	useState,
} from "react";
import {
	defer,
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
import styles from "./Accounting.module.scss";
import { AccountingClient } from "./proto/AccountingServiceClientPb";
import { type Item, NewItem } from "./proto/accounting_pb";
import { formatTimestamp } from "./time";

type TextFieldChangeEventHandler = FormEventHandler<
	HTMLInputElement | HTMLTextAreaElement
>;
type TextFieldChangeEvent = FormEvent<HTMLInputElement | HTMLTextAreaElement>;

export default function Accounting() {
	const [onNameChange, setOnNameChange] =
		useState<TextFieldChangeEventHandler>();
	const [onIncomeChange, setOnIncomeChange] =
		useState<TextFieldChangeEventHandler>();
	const [onExpenseChange, setOnExpenseChange] =
		useState<TextFieldChangeEventHandler>();
	const [onAdd, setOnAdd] = useState<() => void>();
	const [name, setName] = useState<string>("");
	const [income, setIncome] = useState<string>("0");
	const [expense, setExpense] = useState<string>("0");
	const [items, setItems] = useState<Item[]>();
	useEffect(() => {
		const bye$ = new Subject();
		const accountingService = new AccountingClient("/api");
		const nameChange$ = new Subject<string>();
		const incomeChange$ = new Subject<string>();
		const expenseChange$ = new Subject<string>();
		const add$ = new Subject();
		setOnNameChange(
			() => (event: TextFieldChangeEvent) =>
				nameChange$.next(event.currentTarget.value),
		);
		setOnIncomeChange(
			() => (event: TextFieldChangeEvent) =>
				incomeChange$.next(event.currentTarget.value),
		);
		setOnExpenseChange(
			() => (event: TextFieldChangeEvent) =>
				expenseChange$.next(event.currentTarget.value),
		);
		setOnAdd(() => () => add$.next(undefined));
		const reset$: Observable<unknown> = defer(() => addResult$);
		const name$ = nameChange$.pipe(
			startWith(""),
			mergeWith(reset$.pipe(map(() => ""))),
		);
		const income$ = incomeChange$.pipe(
			startWith("0"),
			mergeWith(reset$.pipe(map(() => "0"))),
		);
		const expense$ = expenseChange$.pipe(
			startWith("0"),
			mergeWith(reset$.pipe(map(() => "0"))),
		);
		const addResult$ = add$.pipe(
			withLatestFrom(name$, expense$, income$),
			switchMap(([_, name, expense, income]) => {
				const newItem = new NewItem();
				newItem.setName(name);
				newItem.setExpense(expense);
				newItem.setIncome(income);
				return accountingService.add(newItem);
			}),
			share(),
		);
		const items$ = addResult$.pipe(
			startWith(undefined),
			switchMap(() => accountingService.list(new Empty())),
			map((list) => list.getItemsList()),
			share(),
		);
		name$.pipe(takeUntil(bye$)).subscribe(setName);
		income$.pipe(takeUntil(bye$)).subscribe(setIncome);
		expense$.pipe(takeUntil(bye$)).subscribe(setExpense);
		items$.pipe(takeUntil(bye$)).subscribe(setItems);
		return () => bye$.next(undefined);
	}, []);
	return (
		<Container>
			<Grid container>
				<Grid container gap={1} flexGrow={1}>
					<TextField
						label="項目"
						value={name}
						sx={{ fontSize: 40, flexGrow: 1 }}
						className={styles.grow}
						onChange={onNameChange}
					/>
					<TextField
						label="支出"
						value={expense}
						sx={{ fontSize: 40, flexGrow: 1 }}
						slotProps={{
							htmlInput: {
								sx: { textAlign: "end", color: "#e18b8b", fontWeight: 900 },
							},
						}}
						onChange={onExpenseChange}
					/>
					<TextField
						label="收入"
						value={income}
						sx={{ fontSize: 40, flexGrow: 1 }}
						slotProps={{
							htmlInput: {
								sx: { textAlign: "end", color: "#56b56f", fontWeight: 900 },
							},
						}}
						onChange={onIncomeChange}
					/>
					<Button color="primary" onClick={onAdd}>
						<AddCircleOutlineIcon />
						新增
					</Button>
				</Grid>
				<Table>
					<TableHead>
						<TableRow>
							<TableCell>項目</TableCell>
							<TableCell>支出</TableCell>
							<TableCell>收入</TableCell>
							<TableCell>時間</TableCell>
						</TableRow>
					</TableHead>
					<TableBody>
						{items?.map((item) => (
							<TableRow key={item.getId()}>
								<TableCell>{item.getName()} </TableCell>
								<TableCell>{item.getExpense()} </TableCell>
								<TableCell>{item.getIncome()} </TableCell>
								<TableCell>{formatTimestamp(item.getCreatedAt())} </TableCell>
							</TableRow>
						))}
					</TableBody>
				</Table>
			</Grid>
		</Container>
	);
}
