import CheckIcon from "@mui/icons-material/Check";
import CloseIcon from "@mui/icons-material/Close";
import EditIcon from "@mui/icons-material/Edit";
import Box from "@mui/material/Box";
import Button from "@mui/material/Button";
import Grid from "@mui/material/Grid";
import IconButton from "@mui/material/IconButton";
import TableCell from "@mui/material/TableCell";
import TableRow from "@mui/material/TableRow";
import TextField from "@mui/material/TextField";
import Typography from "@mui/material/Typography";
import { DateTimePicker } from "@mui/x-date-pickers/DateTimePicker";
import { Timestamp } from "google-protobuf/google/protobuf/timestamp_pb";
import { type ChangeEvent, useCallback, useState } from "react";
import MoneyInput from "./MoneyInput";
import {
	Amount,
	AmountType,
	type Item,
	UpdateItemRequest,
} from "./proto/accounting_pb";
import { formatTimestamp } from "./time";

interface Props {
	item: Item;
	onDeleteItem?(): void;
	onUpdateItem?(request: UpdateItemRequest): void;
}

export default function AccountingItemRow({
	item,
	onDeleteItem,
	onUpdateItem,
}: Props) {
	const [editing, setEditing] = useState(false);
	const [update, setUpdate] = useState<UpdateItemRequest>();
	const submitUpdate = useCallback(() => {
		if (update) {
			onUpdateItem?.(update);
		}
		setUpdate(undefined);
		setEditing(false);
	}, [update, onUpdateItem]);
	const startEdit = useCallback(() => {
		const request = new UpdateItemRequest();
		request.setId(item.getId());
		setUpdate(request);
		setEditing(true);
	}, [item]);
	const cancelEdit = useCallback(() => {
		setUpdate(undefined);
		setEditing(false);
	}, []);
	const onNameChange = useCallback((event: ChangeEvent<HTMLInputElement>) => {
		setUpdate((state) => {
			state?.setName(event.target.value);
			return state;
		});
	}, []);
	const onAmountChange = useCallback(
		(amountString: string) => {
			setUpdate((state) => {
				const amount = new Amount();
				amount.setCurrency(item.getAmount()?.getCurrency() ?? "TWD");
				amount.setAmount(amountString);
				state?.setAmount(amount);
				return state;
			});
		},
		[item],
	);
	const onOccurredAtChange = useCallback((date: Date | null) => {
		if (!date) {
			return;
		}
		setUpdate((state) => {
			state?.setOccurredAt(Timestamp.fromDate(date));
			return state;
		});
	}, []);
	return (
		<TableRow key={item.getId()}>
			<TableCell>
				{editing ? (
					<TextField
						label="項目"
						defaultValue={item.getName()}
						onChange={onNameChange}
					/>
				) : (
					item.getName()
				)}
			</TableCell>
			<TableCell>
				{editing ? (
					<MoneyInput
						defaultValue={item.getAmount()?.getAmount()}
						onAmountChange={onAmountChange}
					/>
				) : (
					<Typography
						sx={{
							fontWeight: 900,
							color:
								item.getType() === AmountType.EXPENSE ? "#e18b8b" : "#56b56f",
						}}
					>
						{item.getAmount()?.getAmount()}
					</Typography>
				)}
			</TableCell>
			<TableCell>{item.getAmount()?.getCurrency()}</TableCell>
			<TableCell onClick={() => setEditing(true)}>
				{editing ? (
					<DateTimePicker
						label="時間"
						defaultValue={item.getOccurredAt()?.toDate()}
						onChange={onOccurredAtChange}
					/>
				) : (
					formatTimestamp(item.getOccurredAt())
				)}
			</TableCell>
			<TableCell>
				<Grid container justifyContent="space-between">
					<Button
						variant="outlined"
						color="error"
						onClick={() => onDeleteItem?.()}
					>
						刪除
					</Button>
					{!editing && (
						<IconButton onClick={startEdit}>
							<EditIcon />
						</IconButton>
					)}
					{editing && (
						<Box>
							<IconButton onClick={submitUpdate} color="success">
								<CheckIcon />
							</IconButton>
							<IconButton onClick={cancelEdit} color="error">
								<CloseIcon />
							</IconButton>
						</Box>
					)}
				</Grid>
			</TableCell>
		</TableRow>
	);
}
