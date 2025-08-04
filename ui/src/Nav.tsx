import Box from "@mui/material/Box";
import Container from "@mui/material/Container";
import Tab from "@mui/material/Tab";
import Tabs from "@mui/material/Tabs";
import Typography from "@mui/material/Typography";
import type { Response } from "./GoogleSignIn";
import GoogleSignIn from "./GoogleSignIn";
import LinkTab from "./LinkTab";
import { useRouteMatchCurrentTab } from "./muiutils";

interface Props {
	username?: string;
	promptLogin?: boolean;
	onLogin(response: Response): void;
}

export default function Nav({ username, promptLogin, onLogin }: Props) {
	const currentTab = useRouteMatchCurrentTab(["/todo/*"]);
	return (
		<Box component="nav" sx={{ marginBottom: 2, display: "flex" }}>
			<Typography variant="h5" p={1}>
				AccountCat
			</Typography>
			<Container sx={{ flexGrow: 1 }}>
				<Tabs value={currentTab}>
					<LinkTab to="/todo" value="/todo/*" label="Todo" />
					<Tab value="/finance" label="Finance" disabled />
				</Tabs>
			</Container>

			{username && <span>Hello, {username}</span>}
			{promptLogin && <GoogleSignIn loginCallback={onLogin} />}
		</Box>
	);
}
