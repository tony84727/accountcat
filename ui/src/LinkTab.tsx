import Tab, { type TabProps } from "@mui/material/Tab";
import { Link } from "react-router";

interface Props extends TabProps {
	to: string;
}
export default function LinkTab(props: Props) {
	return <Tab component={Link} {...props} />;
}
