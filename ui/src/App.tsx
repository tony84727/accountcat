import "normalize.css";
import Nav from "./Nav";
import GoogleSignIn from "./GoogleSignIn";

const App = () => {
	return (
		<div>
			<Nav />
			<GoogleSignIn loginCallback={console.log} />
		</div>
	);
};

export default App;
