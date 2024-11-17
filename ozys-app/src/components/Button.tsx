interface ButtonProps {
  children: React.ReactNode
}

export default function Button({ children, ...props }: ButtonProps) {
  return (
    <button className='bg-slate-100 p-8'>
      <h1>{children}</h1>
    </button>
  )
}
