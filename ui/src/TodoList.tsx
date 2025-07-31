import Container from "@mui/material/Container";
import Grid from "@mui/material/Grid";
import Tabs from "@mui/material/Tabs";
import LinkTab from "./LinkTab";
import { useRouteMatch } from "./muiutils";
export default function TodoList() {
	const routeMatch = useRouteMatch(["/todo/history", "/todo"]);
	const currentTab = routeMatch?.pattern?.path;
	return (
		<Container>
			<Grid container>
				<Grid>
					<Tabs value={currentTab} orientation="vertical">
						<LinkTab label="Todo" to="/todo" value="/todo" />
						<LinkTab label="History" to="/todo/history" value="/todo/history" />
					</Tabs>
				</Grid>
				<Grid>Items</Grid>
			</Grid>
		</Container>
	);
}
