interface Page {
	to: string;
	route: string;
	label: string;
}

const administratorPage: Page[] = [
	{
		to: "/instance-settings",
		route: "/instance-settings/*",
		label: "系統設定",
	},
];

const normalPages: Page[] = [
	{
		to: "/accounting",
		route: "/accounting/*",
		label: "記帳",
	},
	{
		to: "/insight",
		route: "/insight/*",
		label: "財務分析",
	},
	{
		to: "/todo",
		route: "/todo/*",
		label: "代辦事項",
	},
];

export default function pages(isAdmin?: boolean): Page[] {
	return isAdmin ? [...normalPages, ...administratorPage] : normalPages;
}
