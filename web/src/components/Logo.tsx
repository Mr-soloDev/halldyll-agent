import './Logo.css'

interface LogoProps {
  size?: number
}

export function Logo({ size = 32 }: LogoProps) {
  return (
    <div className="logo" style={{ width: size, height: size }}>
      <svg viewBox="0 0 100 100" fill="none" xmlns="http://www.w3.org/2000/svg">
        {/* Face circle */}
        <circle cx="50" cy="50" r="45" fill="#6366f1" />

        {/* Eyes */}
        <ellipse cx="35" cy="42" rx="6" ry="7" fill="white" />
        <ellipse cx="65" cy="42" rx="6" ry="7" fill="white" />
        <circle cx="36" cy="43" r="3" fill="#1a1a1a" />
        <circle cx="66" cy="43" r="3" fill="#1a1a1a" />

        {/* Smile */}
        <path
          d="M30 58 Q50 78 70 58"
          stroke="white"
          strokeWidth="5"
          strokeLinecap="round"
          fill="none"
        />

        {/* Cheeks */}
        <circle cx="22" cy="55" r="6" fill="#818cf8" opacity="0.6" />
        <circle cx="78" cy="55" r="6" fill="#818cf8" opacity="0.6" />
      </svg>
    </div>
  )
}
