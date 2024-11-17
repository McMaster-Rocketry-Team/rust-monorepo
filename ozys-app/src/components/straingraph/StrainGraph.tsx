import { useTabAtom } from '../../workspace/useTabAtom'

export const StrainGraph = () => {
  const [text, setText] = useTabAtom('text', 'hello')

  return (
    <div>
      <p>Strain Graph</p>
      <input
        className='border border-gray-400'
        type='text'
        value={text}
        onChange={(e) => setText(e.target.value)}
      />
    </div>
  )
}
