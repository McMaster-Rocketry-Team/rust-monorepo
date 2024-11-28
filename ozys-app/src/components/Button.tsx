
export default function Button({ children }: {children : React.ReactNode}) {
  return (
    <button className='bg-slate-100 p-8'>
      <h1>{children}</h1>
    </button>
  )
}
