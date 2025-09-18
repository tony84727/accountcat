import Button from "@mui/material/Button";
import Container from "@mui/material/Container";
import Paper from "@mui/material/Paper";
import Stack from "@mui/material/Stack";
import TextField from "@mui/material/TextField";
import Typography from "@mui/material/Typography";

export default function InstanceSetting() {
	return (
		<Container>
			<Paper elevation={1}>
				<Stack padding={1}>
					<Typography fontWeight="bold" padding={1}>
						公告
					</Typography>
					<TextField label="訊息" variant="outlined" />
					<Button>儲存</Button>
				</Stack>
			</Paper>
		</Container>
	);
}
