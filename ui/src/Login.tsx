import Container from "@mui/material/Container";
import Typography from "@mui/material/Typography";
import GoogleSignInButton from "./GoogleSignInButton";

export default function Login() {
	return (
		<Container>
			<Typography variant="h6">請先透過Google登入</Typography>
			<GoogleSignInButton />
		</Container>
	);
}
