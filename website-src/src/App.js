import styles from "./pretty/App.module.css"
import { useState, useEffect } from "react";
import axios from 'axios';

function App() {
    const [data, setData] = useState(JSON.parse("{}"));
    const [temperature, setTemperature] = useState("Loading...");

    function sleep(ms) {
        return new Promise(resolve => setTimeout(resolve, ms));
    }


    useEffect(() => {
        async function fetchData() {

            let url = window.location.href;
            //let url = 'http://192.168.1.147:7667/';  //for dev purpose, npm server runs on port 3000, actually port would be 7667

            axios.get(url + 'data/temp').then((response) => { setTemperature(response.data) }).catch((error) => { console.log(error) });

            axios.get(url + 'data').then((response) => {
                setData(response.data)
            }).catch((error) => { console.log(error) });
            console.log("sleep")
            await sleep(1000);
        }
    
        fetchData()
      }, []);

    return (
        <div className={styles.home}>
            <div className={styles.info}>Temperature: {temperature}</div>
            <div className={styles.info}>{JSON.stringify(data)}</div>
        </div>
    );
}

export default App;
