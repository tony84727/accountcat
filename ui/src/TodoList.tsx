import Tabs from "@mui/material/Tabs";
import LinkTab from "./LinkTab";
import { useRouteMatch } from "./muiutils";
export default function TodoList() {
	const routeMatch = useRouteMatch(["/todo/history", "/todo"]);
	const currentTab = routeMatch?.pattern?.path;
	return (
		<Tabs value={currentTab}>
			<LinkTab label="History" to="/todo/history" value="/todo/history" />
			<LinkTab label="Todo" to="/todo" value="/todo" />
		</Tabs>
	);
}
