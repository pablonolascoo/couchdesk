// Fondo 3D: un modelo GLB/GLTF centrado que gira despacio.
// Este archivo (y three.js) solo se descarga cuando se elige este fondo.
import * as THREE from 'three'
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js'

export interface ModelScene {
  setPaused(paused: boolean): void
  dispose(): void
}

interface Options {
  /** Dirección del modelo que puede abrir la app */
  url: string
  /** Para un .gltf local: convierte sus archivos vecinos (.bin, texturas) en direcciones válidas */
  resolveRelative?: (relative: string) => string
  onError(): void
}

const FPS = 30
const REL = 'consola-rel:/'

export function mountModel(container: HTMLElement, { url, resolveRelative, onError }: Options): ModelScene {
  const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true, powerPreference: 'low-power' })
  renderer.setPixelRatio(Math.min(1, window.devicePixelRatio))
  renderer.setClearColor(0x000000, 0)
  container.appendChild(renderer.domElement)

  const scene = new THREE.Scene()
  const camera = new THREE.PerspectiveCamera(35, 1, 0.01, 1000)
  // Luz suave y cálida, como el resto de la consola
  scene.add(new THREE.HemisphereLight(0xfff6ea, 0xb8a58c, 2.2))
  const sun = new THREE.DirectionalLight(0xffe9cc, 1.6)
  sun.position.set(3, 5, 4)
  scene.add(sun)

  const pivot = new THREE.Group()
  scene.add(pivot)

  const resize = () => {
    const w = container.clientWidth || window.innerWidth
    const h = container.clientHeight || window.innerHeight
    renderer.setSize(w, h, false)
    camera.aspect = w / h
    camera.updateProjectionMatrix()
  }
  resize()
  window.addEventListener('resize', resize)

  let paused = false
  let disposed = false
  let raf = 0
  let last = 0
  const loop = (t: number) => {
    raf = requestAnimationFrame(loop)
    if (t - last < 1000 / FPS - 2) return // como mucho 30 imágenes por segundo
    const dt = last ? Math.min(0.1, (t - last) / 1000) : 0
    last = t
    pivot.rotation.y += dt * 0.18 // una vuelta cada ~35 s
    renderer.render(scene, camera)
  }
  const start = () => { if (!raf && !paused && !disposed) { last = 0; raf = requestAnimationFrame(loop) } }
  const stop = () => { cancelAnimationFrame(raf); raf = 0 }

  const manager = new THREE.LoadingManager()
  if (resolveRelative) manager.setURLModifier((u) => (u.startsWith(REL) ? resolveRelative(decodeURIComponent(u.slice(REL.length))) : u))
  const loader = new GLTFLoader(manager)
  if (resolveRelative) loader.setResourcePath(REL)

  loader.load(
    url,
    (gltf) => {
      if (disposed) return disposeObject(gltf.scene)
      const model = gltf.scene
      // Centrar y encajar en la vista, sea cual sea su tamaño
      const box = new THREE.Box3().setFromObject(model)
      const size = box.getSize(new THREE.Vector3())
      const center = box.getCenter(new THREE.Vector3())
      model.position.sub(center)
      pivot.add(model)
      const radius = Math.max(size.length() / 2, 1e-3)
      const fit = radius / Math.sin(THREE.MathUtils.degToRad(camera.fov / 2))
      camera.position.set(0, radius * 0.35, fit * 1.35)
      camera.near = fit / 100
      camera.far = fit * 10
      camera.lookAt(0, 0, 0)
      camera.updateProjectionMatrix()
      start()
    },
    undefined,
    () => { if (!disposed) onError() },
  )

  return {
    setPaused(p) {
      paused = p
      if (p) stop()
      else start()
    },
    dispose() {
      disposed = true
      stop()
      window.removeEventListener('resize', resize)
      disposeObject(scene)
      renderer.dispose()
      renderer.forceContextLoss()
      renderer.domElement.remove()
    },
  }
}

function disposeObject(root: THREE.Object3D) {
  root.traverse((o) => {
    const mesh = o as THREE.Mesh
    mesh.geometry?.dispose()
    const mats = Array.isArray(mesh.material) ? mesh.material : mesh.material ? [mesh.material] : []
    for (const m of mats) {
      for (const v of Object.values(m)) if (v instanceof THREE.Texture) v.dispose()
      m.dispose()
    }
  })
}
