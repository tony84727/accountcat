import Box from "@mui/material/Box";
import Divider from "@mui/material/Divider";
import Drawer, { type DrawerProps } from "@mui/material/Drawer";
import MenuItem from "@mui/material/MenuItem";
import MenuList from "@mui/material/MenuList";
import Toolbar from "@mui/material/Toolbar";
import Typography from "@mui/material/Typography";
import classNames from "classnames";
import { useCallback, useContext, useMemo } from "react";
import { Link } from "react-router";
import GsiContext from "./GsiContext";
import styles from "./MenuDrawer.module.scss";
import { useRouteMatchCurrentTab } from "./muiutils";

interface Props extends DrawerProps {
	onClose?(): void;
}

interface NavMenuItemProps {
	to: string;
	selected?: boolean;
	onClick?(): void;
	label: string;
}

function NavMenuItem({ to, selected, onClick, label }: NavMenuItemProps) {
	return (
		<Link to={to} className={classNames([styles.cleanLink, styles.blackText])}>
			<MenuItem selected={selected} onClick={onClick}>
				<Typography variant="body2">{label}</Typography>
			</MenuItem>
		</Link>
	);
}
export default function MenuDrawer({ onClose, ...drawerProps }: Props) {
	const currentTab = useRouteMatchCurrentTab(["/todo/*", "/accounting/*"]);
	const menuClicked = useCallback(() => onClose?.(), [onClose]);
	const gsi = useContext(GsiContext);
	const username = useMemo(() => gsi.username, [gsi.username]);
	return (
		<Drawer {...drawerProps} onClose={onClose}>
			<Toolbar sx={{ minWidth: "190px" }}>
				<Typography variant="h6">AccountCat</Typography>
			</Toolbar>
			<Divider />
			<Box
				sx={{
					display: "flex",
					flexDirection: "column",
					justifyContent: "space-between",
				}}
			>
				<MenuList>
					<NavMenuItem
						to="/accounting"
						onClick={menuClicked}
						selected={currentTab === "/accounting/*"}
						label="記帳"
					/>
					<NavMenuItem
						to="/todo"
						onClick={menuClicked}
						selected={currentTab === "/todo/*"}
						label="代辦事項"
					/>
				</MenuList>
				{username && (
					<>
						<Divider />
						<Typography sx={{ alignSelf: "center", padding: 2 }}>
							您好，{username}
						</Typography>
					</>
				)}
			</Box>
		</Drawer>
	);
}
