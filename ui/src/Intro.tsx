import GitHubIcon from "@mui/icons-material/GitHub";
import Box from "@mui/material/Box";
import Button from "@mui/material/Button";
import Grid from "@mui/material/Grid";
import Typography from "@mui/material/Typography";
import { useContext } from "react";
import Logo from "./assets/logo.png";
import GoogleSignInButton from "./GoogleSignInButton";
import GsiContext from "./GsiContext";
export default function Intro() {
	const gsiContext = useContext(GsiContext);
	return (
		<Grid
			container
			justifyContent="center"
			flexDirection="row"
			gap={4}
			flexWrap="nowrap"
		>
			<Box
				component="img"
				src={Logo}
				sx={{
					height: "auto",
					maxWidth: {
						xs: "128px",
						sm: "256px",
					},
				}}
				alt="Accountcat Logo"
			/>
			<Grid container alignSelf="center" flexShrink={1} flexDirection="column">
				<Typography
					color="textPrimary"
					sx={{
						fontSize: {
							xs: 32,
							sm: 64,
						},
					}}
				>
					Accountcat
				</Typography>
				<Grid container alignItems="center">
					<Typography color="textPrimary">記帳小幫手</Typography>
					<Button href="https://github.com/tony84727/accountcat">
						<GitHubIcon></GitHubIcon>
						原始碼
					</Button>
				</Grid>
				{!gsiContext.username && (
					<div>
						<Typography>尚未登入，請先登入</Typography>
						<GoogleSignInButton />
					</div>
				)}
			</Grid>
		</Grid>
	);
}
