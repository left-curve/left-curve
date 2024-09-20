const solid = {
  default: "bg-gray-200 text-gray-900",
  white: "bg-white text-gray-900",
  purple: "bg-purple-100 text-purple-900",
  green: "bg-green-100 text-green-900",
  danger: "bg-danger-500 text-white",
  sand: "bg-sand-100 text-sand-900",
};

const shadow = {
  default: "shadow-lg shadow-gray-200/50 bg-gray-200 text-gray-900",
  purple: "shadow-lg shadow-purple-100/50 bg-purple-100 text-purple-900",
  green: "shadow-lg shadow-green-100/50 bg-green-100 text-green-900",
  danger: "shadow-lg shadow-red-100/50 bg-red-100 text-red-900",
  sand: "shadow-lg shadow-sand-100/50 bg-sand-100 text-sand-900",
};

const outline = {
  default: "bg-transparent border-gray-200 text-gray-900",
  purple: "bg-transparent border-purple-100 text-purple-900",
  green: "bg-transparent border-purple-100 text-purple-900",
  danger: "bg-transparent border-purple-100 text-purple-900",
  sand: "bg-transparent border-purple-100 text-purple-900",
};

const flat = {
  // "bg-secondary/20 text-secondary-700"
  default: "bg-gray-200/40 hover:bg-gray-200 text-gray-800",
  purple: "bg-purple-100/40 hover:bg-purple-100 text-purple-900",
  green: "bg-green-100/40 hover:bg-green-100 text-green-900",
  danger: "bg-danger-100/40 hover:bg-danger-100 text-danger-900",
  sand: "bg-sand-100/40 hover:bg-sand-100 text-sand-900",
};

const faded = {
  default: "border-gray-200 bg-gray-100 text-gray-900",
  purple: "border-purple-100 bg-purple-100/60 text-purple-900",
  green: "border-green-100 bg-green-100/60 text-green-900",
  danger: "border-danger-100 bg-danger-100/60 text-danger-900",
  sand: "border-sand-100 bg-sand-100/60 text-sand-900 ",
};

const light = {
  default: "bg-transparent text-gray-900 hover:bg-gray-200/20",
  purple: "bg-transparent text-purple-900 hover:bg-purple-100/20",
  green: "bg-transparent text-green-900 hover:bg-green-100/20",
  danger: "bg-transparent text-danger-900 hover:bg-danger-100/20",
  sand: "bg-transparent text-sand-900 hover:bg-sand-100/20",
};

const dark = {
  default: "bg-gray-500 text-gray-100 hover:bg-gray-200/20 hover:text-gray-900",
  purple: "bg-purple-800 text-purple-100 hover:bg-purple-100/20 hover:text-purple-900",
  green: "bg-green-800 text-green-100 hover:bg-green-100/20 hover:text-green-900",
  danger: "bg-danger-800 text-danger-100 hover:bg-danger-100/20 hover:text-danger-900",
  sand: "bg-sand-800 text-sand-100 hover:bg-sand-100/20 hover:text-sand-900",
};

const ghost = {
  default: "border-gray-500 text-gray-500 hover:bg-gray-500 hover:text-white",
  purple: "border-purple-900 text-purple-900 hover:bg-purple-900 hover:text-purple-100",
  green: "border-green-900 text-green-900 hover:bg-green-900 hover:text-green-100",
  danger: "border-danger-900/60 text-danger-900 hover:bg-danger-900 hover:text-danger-100",
  sand: "border-sand-900/60 text-sand-900 hover:bg-sand-900 hover:text-sand-100",
};

export const colorVariants = {
  solid,
  shadow,
  outline,
  flat,
  faded,
  light,
  dark,
  ghost,
};
