import React from "react";

const HandyTextLogo = ({
  width,
  height,
  className,
}: {
  width?: number;
  height?: number;
  className?: string;
}) => {
  const style = {
    width: width ? `${width}px` : undefined,
    height: height ? `${height}px` : undefined,
  };

  return (
    <div
      className={`select-none ${className ?? ""}`.trim()}
      style={style}
      aria-label="Shadow Word"
    >
      <div className="text-[2rem] font-black uppercase tracking-[0.22em] leading-none text-logo-primary">
        Shadow
      </div>
      <div className="mt-2 text-[0.95rem] font-semibold uppercase tracking-[0.42em] leading-none text-text/75">
        Word
      </div>
    </div>
  );
};

export default HandyTextLogo;
