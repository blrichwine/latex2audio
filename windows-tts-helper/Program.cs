using System;
using System.IO;
using System.Linq;
using System.Speech.Synthesis;

internal static class Program
{
    static int Main(string[] args)
    {
        try
        {
            if (args.Length == 0 || args.Contains("--help") || args.Contains("-h"))
            {
                PrintUsage();
                return 1;
            }

            if (args.Contains("--list-voices"))
            {
                using var voiceSynth = new SpeechSynthesizer();
                foreach (var voice in voiceSynth.GetInstalledVoices())
                {
                    Console.WriteLine(voice.VoiceInfo.Name);
                }
                return 0;
            }

            string? ssmlFile = GetArg(args, "--ssml-file");
            string? outFile = GetArg(args, "--out");
            string? voiceName = GetArg(args, "--voice");

            if (string.IsNullOrWhiteSpace(ssmlFile) || string.IsNullOrWhiteSpace(outFile))
            {
                Console.Error.WriteLine("Error: --ssml-file and --out are required.");
                PrintUsage();
                return 2;
            }

            if (!File.Exists(ssmlFile))
            {
                Console.Error.WriteLine($"Error: SSML file not found: {ssmlFile}");
                return 3;
            }

            int rate = ParseInt(GetArg(args, "--rate"), defaultValue: 0, min: -10, max: 10);
            int volume = ParseInt(GetArg(args, "--volume"), defaultValue: 100, min: 0, max: 100);

            string ssml = File.ReadAllText(ssmlFile);

            using var synth = new SpeechSynthesizer();

            if (!string.IsNullOrWhiteSpace(voiceName))
            {
                synth.SelectVoice(voiceName);
            }

            synth.Rate = rate;
            synth.Volume = volume;
            synth.SetOutputToWaveFile(outFile);
            synth.SpeakSsml(ssml);

            Console.WriteLine($"Wrote WAV: {outFile}");
            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine("Synthesis failed:");
            Console.Error.WriteLine(ex.ToString());
            return 10;
        }
    }

    static string? GetArg(string[] args, string name)
    {
        for (int i = 0; i < args.Length - 1; i++)
        {
            if (string.Equals(args[i], name, StringComparison.OrdinalIgnoreCase))
            {
                return args[i + 1];
            }
        }
        return null;
    }

    static int ParseInt(string? value, int defaultValue, int min, int max)
    {
        if (!int.TryParse(value, out int parsed))
        {
            return defaultValue;
        }

        if (parsed < min) return min;
        if (parsed > max) return max;
        return parsed;
    }

    static void PrintUsage()
    {
        Console.WriteLine("WinSsmlTts - Minimal Windows SSML-to-WAV helper");
        Console.WriteLine();
        Console.WriteLine("Usage:");
        Console.WriteLine("  WinSsmlTts --list-voices");
        Console.WriteLine("  WinSsmlTts --ssml-file input.ssml --out output.wav [--voice \"Voice Name\"] [--rate 0] [--volume 100]");
        Console.WriteLine();
        Console.WriteLine("Examples:");
        Console.WriteLine("  WinSsmlTts --list-voices");
        Console.WriteLine("  WinSsmlTts --ssml-file math.ssml --out math.wav");
        Console.WriteLine("  WinSsmlTts --ssml-file math.ssml --out math.wav --voice \"Microsoft David Desktop\"");
    }
}
