import ToggleButton from "@mui/material/ToggleButton";
import ToggleButtonGroup from "@mui/material/ToggleButtonGroup";
import { useCallback } from "react";
import { AmountType } from "./proto/accounting_pb";

interface Props {
	value?: AmountType;
	onChange?(amountType: AmountType): void;
}

export default function AmountTypeSwitch({ value, onChange }: Props) {
	const onButtonGroupChange = useCallback(
		(_event: React.MouseEvent<HTMLElement>, amountType: AmountType) =>
			onChange?.(amountType),
		[onChange],
	);
	return (
		<ToggleButtonGroup exclusive onChange={onButtonGroupChange} value={value}>
			<ToggleButton color="secondary" value={AmountType.INCOME}>
				收入
			</ToggleButton>
			<ToggleButton color="primary" value={AmountType.EXPENSE}>
				支出
			</ToggleButton>
		</ToggleButtonGroup>
	);
}
