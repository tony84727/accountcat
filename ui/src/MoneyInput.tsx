import type { SxProps } from "@mui/material/styles";
import TextField from "@mui/material/TextField";
import { type FormEvent, useCallback, useEffect, useState } from "react";
import formatInputNumber from "./formatInputNumber";

interface Props {
	defaultValue?: string;
	onAmountChange?(amount: string): void;
	sx?: SxProps;
}

export default function MoneyInput(props: Props) {
	const [amount, setAmount] = useState<string>();
	useEffect(() => {
		setAmount(formatInputNumber(props.defaultValue ?? ""));
	}, [props.defaultValue]);
	const onAmountChange = useCallback(
		(event: FormEvent<HTMLInputElement | HTMLTextAreaElement>) => {
			const formatted = formatInputNumber(event.currentTarget.value);
			setAmount(formatted);
			props.onAmountChange?.(formatted);
		},
		[props.onAmountChange],
	);
	return (
		<TextField
			label="金額"
			value={amount}
			sx={{ fontSize: 40, ...props.sx }}
			slotProps={{
				htmlInput: {
					sx: { textAlign: "end", fontWeight: 900 },
					inputMode: "decimal",
				},
			}}
			onChange={onAmountChange}
		/>
	);
}
