import Container from "@mui/material/Container";
import Grid from "@mui/material/Grid";
import Tabs from "@mui/material/Tabs";
import LinkTab from "./LinkTab";
import { useRouteMatchCurrentTab } from "./muiutils";
export default function TodoList() {
	const currentTab = useRouteMatchCurrentTab(["/todo/history", "/todo"]);
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
