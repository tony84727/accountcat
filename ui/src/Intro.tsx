import Box from "@mui/material/Box";
import Grid from "@mui/material/Grid";
import Typography from "@mui/material/Typography";
import Logo from "./logo.png";
export default function Intro() {
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
				<Typography color="textPrimary">記帳小幫手</Typography>
			</Grid>
		</Grid>
	);
}
