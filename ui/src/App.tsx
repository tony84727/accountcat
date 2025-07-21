import "normalize.css";
import Nav from "./Nav";

const App = () => {
	return (
		<div>
			<Nav onLogin={() => alert("login wip")} />
		</div>
	);
};

export default App;
