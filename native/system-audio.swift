import Foundation
import ScreenCaptureKit
import AVFoundation
import CoreMedia

final class AudioSink: NSObject, SCStreamOutput, SCStreamDelegate, @unchecked Sendable {
    let destination: URL
    var file: AVAudioFile?
    var failure: String?
    init(_ path: String) { destination = URL(fileURLWithPath: path) }
    func stream(_ stream: SCStream, didStopWithError error: Error) { failure = error.localizedDescription; fputs("Capture error: \(error)\n", stderr) }
    func stream(_ stream: SCStream, didOutputSampleBuffer sampleBuffer: CMSampleBuffer, of type: SCStreamOutputType) {
        guard type == .audio, sampleBuffer.isValid,
              let desc = sampleBuffer.formatDescription else { return }
        let format = AVAudioFormat(cmAudioFormatDescription: desc)
        guard
              let buffer = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: AVAudioFrameCount(sampleBuffer.numSamples)) else { return }
        buffer.frameLength = buffer.frameCapacity
        let result = CMSampleBufferCopyPCMDataIntoAudioBufferList(sampleBuffer, at: 0, frameCount: Int32(buffer.frameLength), into: buffer.mutableAudioBufferList)
        guard result == noErr else { return }
        do {
            if file == nil {
                file = try AVAudioFile(forWriting: destination, settings: [AVFormatIDKey: kAudioFormatLinearPCM, AVSampleRateKey: format.sampleRate, AVNumberOfChannelsKey: format.channelCount, AVLinearPCMBitDepthKey: 16, AVLinearPCMIsFloatKey: false, AVLinearPCMIsBigEndianKey: false], commonFormat: format.commonFormat, interleaved: format.isInterleaved)
            }
            try file?.write(from: buffer)
        } catch { failure = error.localizedDescription }
    }
}
@main struct Capture {
    static func main() async {
        guard CommandLine.arguments.count == 2 else { exit(2) }
        do {
            let content = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: true)
            guard let display = content.displays.first else { throw NSError(domain: "QuickTalk", code: 1, userInfo: [NSLocalizedDescriptionKey:"No display available for computer audio capture"])}
            let sink = AudioSink(CommandLine.arguments[1])
            let config = SCStreamConfiguration()
            config.capturesAudio = true
            config.excludesCurrentProcessAudio = true
            config.sampleRate = 16000
            config.channelCount = 1
            config.width = 2; config.height = 2
            config.minimumFrameInterval = CMTime(value: 1, timescale: 1)
            let stream = SCStream(filter: SCContentFilter(display: display, excludingWindows: []), configuration: config, delegate: sink)
            try stream.addStreamOutput(sink, type: .audio, sampleHandlerQueue: DispatchQueue(label: "quicktalk.system-audio"))
            try await stream.startCapture()
            print("READY"); fflush(stdout)
            await Task.detached { _ = readLine() }.value
            try await stream.stopCapture()
            sink.file = nil
            if let failure = sink.failure { fputs("\(failure)\n",stderr); exit(1) }
        } catch { fputs("\(error.localizedDescription)\n", stderr); exit(1) }
    }
}
