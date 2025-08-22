import AppBar from "@mui/material/AppBar";
import Tabs from "@mui/material/Tabs";
import Toolbar from "@mui/material/Toolbar";
import Typography from "@mui/material/Typography";
import { Link } from "react-router";
import styles from "./Bar.module.scss";
import GoogleSignIn, { type Response } from "./GoogleSignIn";
import LinkTab from "./LinkTab";
import { useRouteMatchCurrentTab } from "./muiutils";

interface Props {
	username?: string;
	promptLogin?: boolean;
	onLogin(response: Response): void;
}

export default function Bar({ username, promptLogin, onLogin }: Props) {
	const currentTab = useRouteMatchCurrentTab(["/todo/*", "/accounting/*"]);
	return (
		<AppBar position="sticky">
			<Toolbar sx={{ display: "flex" }}>
				<Link to="/" className={styles.cleanLink}>
					<Typography variant="h6" sx={{ marginX: 1 }}>
						AccountCat
					</Typography>
				</Link>
				<Tabs
					value={currentTab}
					sx={{
						flexGrow: 1,
						"& .MuiTabs-indicator": {
							backgroundColor: (theme) => theme.palette.secondary.light,
						},
					}}
				>
					<LinkTab
						sx={{ color: "white", "&.Mui-selected": { color: "white" } }}
						to="/accounting"
						value="/accounting/*"
						label="記帳"
					/>
					<LinkTab
						sx={{ color: "white", "&.Mui-selected": { color: "white" } }}
						to="/todo"
						value="/todo/*"
						label="代辦事項"
					/>
				</Tabs>
				{username && <span>您好，{username}</span>}
				{promptLogin && <GoogleSignIn loginCallback={onLogin} />}
			</Toolbar>
		</AppBar>
	);
}
