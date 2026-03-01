interface CardProps {
  children: React.ReactNode;
  className?: string;
  hover?: boolean;
  padding?: 'sm' | 'md' | 'lg';
}

const PADDINGS = { sm: 'p-4', md: 'p-6', lg: 'p-8' } as const;

export function Card({ children, className = '', hover = false, padding = 'md' }: CardProps) {
  return (
    <div className={`
      bg-white rounded-2xl
      shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.06)]
      ${hover
        ? 'transition-shadow duration-200 ease-[cubic-bezier(0.4,0,0.2,1)] hover:shadow-[0_3px_6px_rgba(0,0,0,0.10),0_2px_4px_rgba(0,0,0,0.06)]'
        : ''}
      ${PADDINGS[padding]}
      ${className}
    `}>
      {children}
    </div>
  );
}
