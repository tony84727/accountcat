interface Page {
	to: string;
	route: string;
	label: string;
}

const pages: Page[] = [
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
export default pages;
