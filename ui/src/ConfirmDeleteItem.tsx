import Button from "@mui/material/Button";
import Dialog from "@mui/material/Dialog";
import DialogActions from "@mui/material/DialogActions";
import DialogContent from "@mui/material/DialogContent";
import DialogTitle from "@mui/material/DialogTitle";
import Typography from "@mui/material/Typography";
import { AmountType, type Item } from "./proto/accounting_pb";

interface Props {
	item?: Item;
	onClose?(): void;
	onConfirm?(): void;
}

export default function ConfirmDeleteItem({ item, onClose, onConfirm }: Props) {
	return (
		<Dialog open={!!item} onClose={onClose}>
			<DialogTitle>確認刪除</DialogTitle>
			<DialogContent>
				{item && (
					<Typography>
						確認刪除一筆
						{item.getType() === AmountType.EXPENSE ? "支出" : "收入"}"
						{item.getName()}"?
					</Typography>
				)}
			</DialogContent>
			<DialogActions>
				<Button color="primary" onClick={onClose}>
					取消
				</Button>
				<Button variant="contained" color="error" onClick={onConfirm}>
					刪除
				</Button>
			</DialogActions>
		</Dialog>
	);
}
