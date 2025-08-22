import Drawer from "@mui/material/Drawer";
import List from "@mui/material/List";
import ListItem from "@mui/material/ListItem";
import ListItemText from "@mui/material/ListItemText";
import { Fragment } from "react";
import { Link } from "react-router";

const routes = [
	{
		name: "記帳",
		path: "/accounting",
	},
	{
		name: "代辦事項",
		path: "/todo",
	},
];

export default function AppDrawer() {
	const drawer = (
		<Fragment>
			<List>
				{routes.map(({ name, path }) => (
					<Link key={name} to={path}>
						<ListItem disablePadding>
							<ListItemText primary={name} />
						</ListItem>
					</Link>
				))}
			</List>
		</Fragment>
	);
	return <Drawer>{drawer}</Drawer>;
}
