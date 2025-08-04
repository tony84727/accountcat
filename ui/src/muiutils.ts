import { matchPath, useLocation } from "react-router";

function useRouteMatch(patterns: readonly string[]) {
	const { pathname } = useLocation();

	for (let i = 0; i < patterns.length; i += 1) {
		const pattern = patterns[i];
		const possibleMatch = matchPath(pattern, pathname);
		if (possibleMatch !== null) {
			return possibleMatch;
		}
	}

	return null;
}

export function useRouteMatchCurrentTab(patterns: readonly string[]) {
	return useRouteMatch(patterns)?.pattern?.path;
}
