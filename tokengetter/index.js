const puppeteer = require('puppeteer');

(async () => {
  const args = process.argv;
  const browser = await puppeteer.launch({
    executablePath: process.env.PUPPETEER_EXECUTABLE_PATH,
    headless: 'new',
    args: ['--no-sandbox',
      '--disable-setuid-sandbox',
      '--disable-gpu',
      '--disable-dev-shm-usage',
      '--disable-blink-features=AutomationControlled',
      '--window-size=1280,720']
  });

  const page = await browser.newPage();

  await page.evaluateOnNewDocument(() => {
    Object.defineProperty(navigator, 'webdriver', { get: () => false });
  });
  await page.setUserAgent('Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36');

  // login
  await page.goto('https://essexnorthshore.schoology.com/');

  await page.type('input[type="email"]', `${args[2]}`); // <-- config value
  await page.click('#identifierNext');
  await page.waitForNavigation({ waitUntil: 'networkidle0' });


  await page.type('input[type="password"]', `${args[3]}`); // <-- config value
  await page.click('#passwordNext');

  await page.waitForNavigation({ waitUntil: 'networkidle0' });

  // Cookie time
  let cookies = await browser.cookies()

  const sessCookie = cookies.find(cookies =>
    cookies.name.startsWith("SESS") &&
    cookies.domain === ".essexnorthshore.schoology.com"
  );

  if (sessCookie) {
    let cookie = `${sessCookie.name}=${sessCookie.value}`;
    console.log(cookie);
  }

  browser.close();
})();
