import AddCircleOutlineIcon from "@mui/icons-material/AddCircleOutline";
import Autocomplete from "@mui/material/Autocomplete";
import Button from "@mui/material/Button";
import Container from "@mui/material/Container";
import FormControl from "@mui/material/FormControl";
import Grid from "@mui/material/Grid";
import InputLabel from "@mui/material/InputLabel";
import MenuItem from "@mui/material/MenuItem";
import Select, { type SelectChangeEvent } from "@mui/material/Select";
import Table from "@mui/material/Table";
import TableBody from "@mui/material/TableBody";
import TableCell from "@mui/material/TableCell";
import TableHead from "@mui/material/TableHead";
import TableRow from "@mui/material/TableRow";
import TextField from "@mui/material/TextField";
import classNames from "classnames";
import { Empty } from "google-protobuf/google/protobuf/empty_pb";
import {
	type FormEvent,
	type FormEventHandler,
	type SyntheticEvent,
	useEffect,
	useState,
} from "react";
import {
	combineLatestWith,
	defer,
	from,
	map,
	mergeMap,
	mergeWith,
	type Observable,
	Subject,
	share,
	startWith,
	switchMap,
	takeUntil,
	toArray,
	withLatestFrom,
} from "rxjs";
import styles from "./Accounting.module.scss";
import { AccountingClient } from "./proto/AccountingServiceClientPb";
import {
	Amount,
	AmountType,
	type Item,
	NewItem,
	NewTag,
	TagSearch,
} from "./proto/accounting_pb";
import {
	createCallback,
	createMultiArgumentCallback,
	createNotifier,
} from "./rxjsutils";
import { formatTimestamp } from "./time";

type TextFieldChangeEventHandler = FormEventHandler<
	HTMLInputElement | HTMLTextAreaElement
>;
type TextFieldChangeEvent = FormEvent<HTMLInputElement | HTMLTextAreaElement>;

const extractTextFieldValue = () =>
	map((event: TextFieldChangeEvent) => event.currentTarget.value);

interface TagOption {
	id?: string;
	label: string;
	create?: string;
}

function isNotEmpty<T>(x: T | undefined): x is T {
	return Boolean(x);
}

export default function Accounting() {
	const [onNameChange, setOnNameChange] =
		useState<TextFieldChangeEventHandler>();
	const [onAmountChange, registerOnExpenseChange] =
		useState<TextFieldChangeEventHandler>();
	const [onTagChange, setOnTagChange] =
		useState<(event: SyntheticEvent, selected: TagOption[]) => void>();
	const [onTagInputChange, registerOnTagInputChange] =
		useState<(event: SyntheticEvent, value: string, reason: string) => void>();
	const [onAdd, setOnAdd] = useState<() => void>();
	const [onCurrnecyChange, registerOnCurrencyChange] =
		useState<(event: SelectChangeEvent) => void>();
	const [name, setName] = useState<string>("");
	const [amount, setAmount] = useState<string>("0");
	const [currency, setCurrency] = useState<string>("TWD");
	const [items, setItems] = useState<Item[]>();
	const [currencies, setCurrencies] = useState<string[]>();
	const [selectedTags, setSelectedTags] = useState<TagOption[]>([]);
	const [tagOptions, setTagOptions] = useState<TagOption[]>([]);
	useEffect(() => {
		const bye$ = new Subject();
		const accountingService = new AccountingClient("/api");
		const nameChange$ = createCallback(setOnNameChange).pipe(
			extractTextFieldValue(),
		);
		const amountChange$ = createCallback(registerOnExpenseChange).pipe(
			extractTextFieldValue(),
		);
		const add$ = createNotifier(setOnAdd);
		const selectedTagChange$ = createMultiArgumentCallback(setOnTagChange);
		const onTagInputChange$ = createMultiArgumentCallback(
			registerOnTagInputChange,
		);
		const reset$: Observable<unknown> = defer(() => addResult$);
		const currencyChange$ = createCallback(registerOnCurrencyChange);
		const currencies$ = defer(() =>
			accountingService.listCurrency(new Empty()),
		).pipe(
			map((response) => response.getCodeList()),
			share(),
		);
		const currency$ = currencyChange$.pipe(
			map((e) => e.target.value),
			startWith("TWD"),
		);
		const name$ = nameChange$.pipe(
			startWith(""),
			mergeWith(reset$.pipe(map(() => ""))),
		);
		const amount$ = amountChange$.pipe(
			startWith("0"),
			mergeWith(reset$.pipe(map(() => "0"))),
		);
		const selectedTags$ = selectedTagChange$.pipe(
			map(([, selected]) => selected.filter((x) => !x.create)),
			combineLatestWith(
				defer(() => createTagResult$).pipe(startWith(undefined)),
			),
			map(([selected, newTag]) => [
				...selected,
				...(newTag ?? []).map((t) => ({ label: t.getName(), id: t.getId() })),
			]),
			startWith([]),
		);
		const addResult$ = add$.pipe(
			withLatestFrom(name$, amount$, selectedTags$, currency$),
			switchMap(([_, name, expense, tags, currency]) => {
				const newItem = new NewItem();
				const amount = new Amount();
				amount.setAmount(expense);
				amount.setCurrency(currency);
				newItem.setName(name);
				newItem.setAmount(amount);
				newItem.setTagsList(tags.map((x) => x.id).filter(isNotEmpty));
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
		const tagKeyword$ = onTagInputChange$.pipe(map(([, keyword]) => keyword));
		const completeResults$ = tagKeyword$.pipe(
			mergeWith(defer(() => createTagResult$).pipe(map(() => ""))),
			startWith(""),
			switchMap((keyword) => {
				const search = new TagSearch();
				search.setKeyword(keyword);
				return accountingService.completeTag(search);
			}),
			share(),
		);
		const tagMissing$ = completeResults$.pipe(
			withLatestFrom(tagKeyword$),
			map(([list, keyword]) =>
				list
					.getTagsList()
					.map((t) => t.getName())
					.indexOf(keyword) === -1
					? keyword
					: undefined,
			),
			startWith(undefined),
		);
		const tagOptions$ = completeResults$.pipe(
			map((list) =>
				list
					.getTagsList()
					.map((tag) => ({ id: tag.getId(), label: tag.getName() })),
			),
			combineLatestWith(tagMissing$),
			map(([list, missing]) =>
				missing
					? [
							...list,
							{
								label: `新增標籤"${missing}"`,
								create: missing,
							},
						]
					: list,
			),
		);
		const tagToCreate$ = selectedTagChange$.pipe(
			map(
				([, selected]) =>
					selected.map(({ create }) => create).filter(Boolean) as string[],
			),
		);
		const createTagResult$ = tagToCreate$.pipe(
			mergeMap((tags) =>
				from(tags).pipe(
					mergeMap((tag) => {
						const newTag = new NewTag();
						newTag.setName(tag);
						return accountingService.createTag(newTag);
					}),
					toArray(),
				),
			),
			share(),
		);
		name$.pipe(takeUntil(bye$)).subscribe(setName);
		amount$.pipe(takeUntil(bye$)).subscribe(setAmount);
		items$.pipe(takeUntil(bye$)).subscribe(setItems);
		selectedTags$.pipe(takeUntil(bye$)).subscribe(setSelectedTags);
		tagOptions$.pipe(takeUntil(bye$)).subscribe(setTagOptions);
		currencies$.pipe(takeUntil(bye$)).subscribe(setCurrencies);
		currency$.pipe(takeUntil(bye$)).subscribe(setCurrency);
		return () => bye$.next(undefined);
	}, []);
	return (
		<Container>
			<Grid container>
				<Grid container gap={1} flexGrow={1} direction="column">
					<TextField
						label="項目"
						value={name}
						sx={{ fontSize: 40 }}
						className={styles.grow}
						onChange={onNameChange}
					/>
					<Grid container gap={1}>
						<TextField
							label="金額"
							value={amount}
							sx={{ fontSize: 40 }}
							className={classNames([
								styles.grow,
								styles.amount,
								styles.amountInput,
							])}
							slotProps={{
								htmlInput: {
									sx: { textAlign: "end", fontWeight: 900 },
								},
							}}
							onChange={onAmountChange}
						/>
						<FormControl className={styles.currencySelect}>
							<InputLabel>貨幣</InputLabel>
							<Select
								value={currency}
								labelId="currency-select"
								label="貨幣"
								onChange={onCurrnecyChange}
							>
								{currencies?.map((x) => (
									<MenuItem key={x} value={x}>
										{x}
									</MenuItem>
								))}
							</Select>
						</FormControl>
					</Grid>
					<FormControl>
						<InputLabel></InputLabel>
						<Autocomplete
							multiple
							renderInput={(params) => (
								<TextField {...params} placeholder="標籤" />
							)}
							value={selectedTags}
							options={tagOptions}
							onChange={onTagChange}
							onInputChange={onTagInputChange}
						/>
					</FormControl>
					<Button color="primary" onClick={onAdd} variant="contained">
						<AddCircleOutlineIcon />
						新增
					</Button>
				</Grid>
				<Table>
					<TableHead>
						<TableRow>
							<TableCell>項目</TableCell>
							<TableCell>金額</TableCell>
							<TableCell>幣別</TableCell>
							<TableCell>時間</TableCell>
						</TableRow>
					</TableHead>
					<TableBody>
						{items?.map((item) => (
							<TableRow key={item.getId()}>
								<TableCell>{item.getName()} </TableCell>
								<TableCell
									className={classNames([
										styles.amount,
										{
											[styles.expense]: item.getType() === AmountType.EXPENSE,
											[styles.income]: item.getType() === AmountType.INCOME,
										},
									])}
								>
									{item.getAmount()?.getAmount()}{" "}
								</TableCell>
								<TableCell>{item.getAmount()?.getCurrency()}</TableCell>
								<TableCell>{formatTimestamp(item.getCreatedAt())} </TableCell>
							</TableRow>
						))}
					</TableBody>
				</Table>
			</Grid>
		</Container>
	);
}
