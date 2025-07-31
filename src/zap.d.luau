declare class ZapType
end

declare class ZapNumberType extends ZapType
end

declare class ZapEvent
end

declare zap: {
	boolean: () -> ZapType,

	u8: (min: number?, max: number?) -> ZapNumberType,
	u16: (min: number?, max: number?) -> ZapNumberType,
	u32: (min: number?, max: number?) -> ZapNumberType,

	i8: (min: number?, max: number?) -> ZapNumberType,
	i16: (min: number?, max: number?) -> ZapNumberType,
	i32: (min: number?, max: number?) -> ZapNumberType,

	f32: (min: number?, max: number?) -> ZapNumberType,
	f64: (min: number?, max: number?) -> ZapType,

	nanf32: (min: number?, max: number?) -> ZapNumberType,
	nanf64: (min: number?, max: number?) -> ZapType,

	vector: (ZapNumberType, ZapNumberType, ZapNumberType?) -> ZapType,

	string: {
		binary: (min: number?, max: number?) -> ZapType,
		utf8: (min: number?, max: number?) -> ZapType,
	},

	array: (ZapType, min: number?, max: number?) -> ZapType,
	set: (ZapType, min: number?, max: number?) -> ZapType,
	map: (ZapType, ZapType, min: number?, max: number?) -> ZapType,
	struct: ({ [string]: ZapType }, min: number?, max: number?) -> ZapType,

	event: (from: "server" | "client", { ZapType }) -> ZapEvent,

	options: ({
		casing: "lower" | "camel" | "pascal" | "snake",
		apicheck: "none" | "some" | "full",
	}),
}
