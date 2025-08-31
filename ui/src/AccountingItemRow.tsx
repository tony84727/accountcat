import Button from "@mui/material/Button";
import TableCell from "@mui/material/TableCell";
import TableRow from "@mui/material/TableRow";
import { AmountType, type Item } from "./proto/accounting_pb";
import { formatTimestamp } from "./time";

interface Props {
	item: Item;
	onDeleteItem?(): void;
}

export default function AccountingItemRow({ item, onDeleteItem }: Props) {
	return (
		<TableRow key={item.getId()}>
			<TableCell>{item.getName()} </TableCell>
			<TableCell
				sx={{
					fontWeight: 900,
					color: item.getType() === AmountType.EXPENSE ? "#e18b8b" : "#56b56f",
				}}
			>
				{item.getAmount()?.getAmount()}{" "}
			</TableCell>
			<TableCell>{item.getAmount()?.getCurrency()}</TableCell>
			<TableCell>{formatTimestamp(item.getCreatedAt())} </TableCell>
			<TableCell>
				<Button
					variant="outlined"
					color="error"
					onClick={() => onDeleteItem?.()}
				>
					刪除
				</Button>
			</TableCell>
		</TableRow>
	);
}
