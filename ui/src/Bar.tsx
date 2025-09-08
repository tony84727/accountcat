import MenuIcon from "@mui/icons-material/Menu";
import AppBar from "@mui/material/AppBar";
import IconButton from "@mui/material/IconButton";
import Tabs from "@mui/material/Tabs";
import Toolbar from "@mui/material/Toolbar";
import Typography from "@mui/material/Typography";
import useMediaQuery from "@mui/material/useMediaQuery";
import { useContext, useMemo } from "react";
import { Link } from "react-router";
import styles from "./Bar.module.scss";
import GoogleSignIn from "./GoogleSignIn";
import GsiContext from "./GsiContext";
import LinkTab from "./LinkTab";
import { useRouteMatchCurrentTab } from "./muiutils";
import pages from "./pages";

interface Props {
	openDrawer?(): void;
}

export default function Bar({ openDrawer }: Props) {
	const currentTab = useRouteMatchCurrentTab(pages.map(({ route }) => route));
	const showDrawerButton = useMediaQuery((theme) =>
		theme.breakpoints.down("sm"),
	);
	const gsi = useContext(GsiContext);
	const username = useMemo(() => gsi.username, [gsi.username]);

	return (
		<AppBar position="sticky">
			<Toolbar sx={{ display: "flex" }}>
				{showDrawerButton && (
					<IconButton sx={{ color: "white" }} onClick={openDrawer}>
						<MenuIcon />
					</IconButton>
				)}
				<Link to="/" className={styles.cleanLink}>
					<Typography variant="h6" sx={{ marginX: 1 }}>
						AccountCat
					</Typography>
				</Link>
				{!showDrawerButton && (
					<Tabs
						value={currentTab}
						sx={{
							flexGrow: 1,
							"& .MuiTabs-indicator": {
								backgroundColor: (theme) => theme.palette.secondary.light,
							},
						}}
					>
						{pages.map(({ to, route, label }) => (
							<LinkTab
								key={label}
								sx={{ color: "white", "&.Mui-selected": { color: "white" } }}
								to={to}
								value={route}
								label={label}
							/>
						))}
					</Tabs>
				)}
				{!showDrawerButton && username && <span>您好，{username}</span>}
				{!username && <GoogleSignIn />}
			</Toolbar>
		</AppBar>
	);
}
