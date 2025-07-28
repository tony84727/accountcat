import "normalize.css";
import { Empty } from "google-protobuf/google/protobuf/empty_pb";
import { defer, from } from "rxjs";
import { map, mergeWith, share, startWith, switchMap } from "rxjs/operators";
import type { Response } from "./GoogleSignIn";
import Nav from "./Nav";
import { UserClient } from "./proto/UserServiceClientPb";
import { LoginRequest } from "./proto/user_pb";
import { useObservable, withSubject } from "./rxjsutils";

const userClient = new UserClient("/api");
const [onLogin$, onLogin] = withSubject<Response>();
const username$ = defer(() => userClient.getName(new Empty())).pipe(
	map((response) => response.getName()),
	mergeWith(
		onLogin$.pipe(
			switchMap(({ credential }) => {
				const request = new LoginRequest();
				request.setToken(credential);
				return from(userClient.login(request)).pipe(
					map((response) => response.getName()),
				);
			}),
		),
	),
	share(),
);
const promptLogin$ = username$.pipe(
	map((x) => !x),
	startWith(false),
);

const App = () => {
	const promptLogin = useObservable(promptLogin$);
	const username = useObservable(username$);
	return (
		<div>
			<Nav username={username} onLogin={onLogin} promptLogin={promptLogin} />
		</div>
	);
};

export default App;
