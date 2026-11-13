const path=require('node:path')
const fs=require('node:fs')
const root=path.resolve(__dirname,'..')
function build() {
  require('esbuild').buildSync({entryPoints:[path.join(root,'src/app.js')],outfile:path.join(root,'app.js'),bundle:true,platform:'browser',target:'es2022',legalComments:'inline',logLevel:'warning'})
  require('esbuild').buildSync({entryPoints:[path.join(root,'src/styles.css')],outfile:path.join(root,'app.css'),bundle:true,external:['assets/*'],logLevel:'warning'})
  for(const file of ['index.html','styles.css','assets/app-icon.png','assets/fonts/Manrope.ttf','assets/fonts/Fraunces.ttf','assets/sprout/growing-plant-sprite.png'])if(!fs.existsSync(path.join(root,file)))throw new Error('Missing site asset: '+file)
}
if(require.main===module){build();console.log('Catch site built locally.')}
module.exports={build}
