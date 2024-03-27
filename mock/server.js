import { createServer } from 'node:http'
import { readFile } from 'node:fs/promises'
import { dirname, join } from 'node:path'

const __filename = new URL(import.meta.url).pathname
const __dirname = dirname(__filename)
const mockData = JSON.parse(await readFile(join(__dirname, 'mock-data.json'), 'utf-8'))

createServer(function (req, res) {
  console.log(new Date(), req.method, req.url)
  const url = new URL(req.url, 'http://localhost:8000/')
  if (req.method === 'GET' && url.pathname.match(/\/3.0\/lists\/(.+)\/members\/?/)) {
    res.writeHead(200, { 'Content-Type': 'application/json' })
    const offset = parseInt(url.searchParams.get('offset'), 10) || 0
    const count = parseInt(url.searchParams.get('count'), 10) || 100
    const response = { ...mockData, members: mockData.members.slice(offset, offset + count) }
    console.log(response)
    return res.end(JSON.stringify(response, null, 2))
  } else if (req.method === 'PATCH' && url.pathname.match(/\/3.0\/lists\/(.+)\/members\/(.+)\/?/)) {
    res.writeHead(200, { 'Content-Type': 'application/json' })
    return res.end(JSON.stringify({ "message": "OK" }))
  } else {
    res.writeHead(404, { 'Content-Type': 'application/json' })
    return res.end(JSON.stringify({ "message": "Not Found" }))
  }
}).listen(8000)