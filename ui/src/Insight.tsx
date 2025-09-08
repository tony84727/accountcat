import Container from "@mui/material/Container";
import Divider from "@mui/material/Divider";
import Grid from "@mui/material/Grid";
import Paper from "@mui/material/Paper";
import Stack from "@mui/material/Stack";
import type { Palette, PaletteColor } from "@mui/material/styles";
import Typography from "@mui/material/Typography";

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
	return (
		<Container>
			<Typography variant="h4">近期消費</Typography>
			<Divider sx={{ marginY: 1 }}></Divider>
			<Paper elevation={1} sx={{ padding: 1 }}>
				<Stack>
					<Typography fontWeight="bold">本日摘要(2025/09/08)</Typography>
					<Grid container gap={1}>
						<ColorBlock label="收入" color="success">
							1000
						</ColorBlock>
						<ColorBlock label="支出" color="error">
							100
						</ColorBlock>
					</Grid>
					<Typography variant="subtitle1">共10筆交易</Typography>
				</Stack>
			</Paper>
		</Container>
	);
}
