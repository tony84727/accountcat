import Tab from "@mui/material/Tab";
import Tabs from "@mui/material/Tabs";
import { Link } from "react-router";
import { useRouteMatch } from "./muiutils";
export default function TodoList() {
	const routeMatch = useRouteMatch(["/todo/history", "/todo"]);
	const currentTab = routeMatch?.pattern?.path;
	return (
		<Tabs value={currentTab}>
			<Tab
				label="History"
				component={Link}
				to="/todo/history"
				value="/todo/history"
			/>
			<Tab label="Todo" component={Link} to="/todo" value="/todo" />
		</Tabs>
	);
}
